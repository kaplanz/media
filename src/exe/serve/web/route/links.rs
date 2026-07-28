//! Link routes.

use axum::Extension;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use media::link::{Body, Link, Patch};
use media::logs::Log;
use media::{Item, Meta};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use super::query::Order;
use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Pool, Uuid as DbUuid};
use crate::schema::{links, media as m, tags};

type Record = media::Record<Item>;

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, modify, remove))
        .routes(routes!(list_tags, set_tags))
        .routes(routes!(insert_tag, remove_tag))
        .routes(routes!(list_logs, set_logs, insert_log))
        .routes(routes!(remove_log))
        .layer(Extension(media::Kind::Link))
}

/// Sort field for links.
#[derive(Clone, Copy, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
enum Sort {
    /// Sort by title.
    Title,
    /// Sort by creation time.
    #[default]
    Created,
    /// Sort by last update time.
    Updated,
}

/// Query parameters for listing links.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
    /// Search title (case-insensitive substring).
    q: Option<String>,
    /// Filter by tag.
    tag: Option<String>,
    /// Field to sort by.
    #[param(inline)]
    sort: Option<Sort>,
    /// Sort direction.
    #[param(inline)]
    order: Option<Order>,
    /// Maximum number of results.
    limit: Option<i64>,
    /// Number of results to skip.
    offset: Option<i64>,
}

/// List links.
#[utoipa::path(
    get,
    path = "/",
    tag = "links",
    params(Params),
    responses((status = 200, body = Vec<Record>)),
)]
async fn list(
    State(db): State<Pool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Record>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;

    // Build query
    let mut query = links::table
        .inner_join(m::table)
        .select((links::all_columns, m::created, m::updated))
        .into_boxed();

    // Apply filters
    if let Some(q) = params.q {
        query = query.filter(links::title.like(format!("%{q}%")));
    }
    if let Some(tag) = params.tag {
        query = query.filter(diesel::dsl::exists(
            tags::table
                .filter(tags::media.eq(links::id))
                .filter(tags::label.eq(tag)),
        ));
    }

    // Sort and paginate
    let rows: Vec<(Link, i64, i64)> = {
        let q = match (
            params.sort.unwrap_or_default(),
            params.order.unwrap_or_default(),
        ) {
            (Sort::Title, Order::Asc) => query.order_by(links::title.asc()),
            (Sort::Title, Order::Desc) => query.order_by(links::title.desc()),
            (Sort::Created, Order::Asc) => query.order_by(m::created.asc()),
            (Sort::Created, Order::Desc) => query.order_by(m::created.desc()),
            (Sort::Updated, Order::Asc) => query.order_by(m::updated.asc()),
            (Sort::Updated, Order::Desc) => query.order_by(m::updated.desc()),
        };
        if let Some(limit) = params.limit {
            q.limit(limit)
                .offset(params.offset.unwrap_or(0))
                .load(&mut conn)
                .await
        } else {
            q.load(&mut conn).await
        }
    }
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(Error::from)?;

    // Load tags
    let ids: Vec<DbUuid> = rows.iter().map(|(l, _, _)| l.id.into()).collect();
    let mut tags = super::tags::load_tags_for(&mut conn, &ids).await?;
    let mut logs = super::logs::load_logs_for(&mut conn, &ids).await?;

    let records = rows
        .into_iter()
        .map(|(link, created, updated)| {
            let tags = tags.remove(&link.id).unwrap_or_default();
            let logs = logs.remove(&link.id).unwrap_or_default();
            let item = Item::Link(link);
            Record {
                item,
                meta: Meta { created, updated },
                logs,
                tags,
            }
        })
        .collect();

    Ok(Json(records))
}

/// Fetch a link by ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "links",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Record), (status = 404)),
)]
async fn fetch(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<Json<Record>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);

    // Load item
    let (link, created, updated) = links::table
        .inner_join(m::table)
        .select((links::all_columns, m::created, m::updated))
        .filter(links::id.eq(uid))
        .first::<(Link, i64, i64)>(&mut conn)
        .await
        .optional()
        .map_err(Error::from)?
        .ok_or(Error::NotFound)?;

    // Load tags
    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let logs = super::logs::load_logs_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();

    let item = Item::Link(link);
    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        logs,
        tags,
    }))
}

/// Create a link.
#[utoipa::path(
    post,
    path = "/",
    tag = "links",
    security(("BearerAuth" = [])),
    request_body(content = inline(Body)),
    responses((status = 201, body = Uuid), (status = 500)),
)]
async fn create(
    State(db): State<Pool>,
    Json(body): Json<Body>,
) -> Result<(StatusCode, Json<Uuid>), Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let id = Uuid::new_v4();
    let uid = DbUuid::from(id);
    // NOTE: Rows must be inserted explicitly because there is no database insert trigger.
    conn.transaction(|conn| {
        async move {
            diesel::insert_into(m::table)
                .values((m::id.eq(uid), m::kind.eq("link")))
                .execute(conn)
                .await?;
            diesel::insert_into(links::table)
                .values((
                    links::id.eq(uid),
                    links::url.eq(&body.url),
                    links::title.eq(&body.title),
                ))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .inspect_err(|err: &diesel::result::Error| tracing::error!("{err}"))
    .map_err(Error::from)?;
    Ok((StatusCode::CREATED, Json(id)))
}

/// Update a link.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "links",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Body)),
    responses((status = 200, body = inline(Record)), (status = 404)),
)]
async fn update(
    State(db): State<Pool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Body>,
) -> Result<Json<Record>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    let n = diesel::update(links::table.filter(links::id.eq(uid)))
        .set((links::url.eq(&body.url), links::title.eq(&body.title)))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    if n == 0 {
        return Err(Error::NotFound);
    }
    diesel::update(m::table.filter(m::id.eq(uid)))
        .set(m::updated.eq(db::timestamp()))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    // Load updated item
    let (link, created, updated) = links::table
        .inner_join(m::table)
        .select((links::all_columns, m::created, m::updated))
        .filter(links::id.eq(uid))
        .first::<(Link, i64, i64)>(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    // Load tags
    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let logs = super::logs::load_logs_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let item = Item::Link(link);
    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        logs,
        tags,
    }))
}

/// Modify a link.
#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "links",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Patch)),
    responses((status = 200, body = inline(Record)), (status = 404)),
)]
async fn modify(
    State(db): State<Pool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Patch>,
) -> Result<Json<Record>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    // Apply present fields
    if !body.is_empty() {
        let n = diesel::update(links::table.filter(links::id.eq(uid)))
            .set((
                body.url.map(|v| links::url.eq(v)),
                body.title.map(|v| links::title.eq(v)),
            ))
            .execute(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
        if n == 0 {
            return Err(Error::NotFound);
        }
        diesel::update(m::table.filter(m::id.eq(uid)))
            .set(m::updated.eq(db::timestamp()))
            .execute(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
    }
    // Load updated item
    let (link, created, updated) = links::table
        .inner_join(m::table)
        .select((links::all_columns, m::created, m::updated))
        .filter(links::id.eq(uid))
        .first::<(Link, i64, i64)>(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    // Load tags
    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let logs = super::logs::load_logs_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let item = Item::Link(link);
    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        logs,
        tags,
    }))
}

/// Delete a link.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "links",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    responses((status = 204), (status = 404)),
)]
async fn remove(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<StatusCode, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let n = diesel::delete(m::table.filter(m::id.eq(DbUuid::from(id))))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    if n > 0 {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(Error::NotFound)
    }
}

/// List tags for a link.
#[utoipa::path(
    get,
    path = "/{id}/tags",
    tag = "links",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
async fn list_tags(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<Uuid>,
) -> Result<Json<Vec<String>>, Error> {
    super::tags::list(state, kind, path).await
}

/// Replace tags for a link.
#[utoipa::path(
    put,
    path = "/{id}/tags",
    tag = "links",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body = Vec<String>,
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
async fn set_tags(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<Uuid>,
    body: Json<Vec<String>>,
) -> Result<Json<Vec<String>>, Error> {
    super::tags::set(state, kind, path, body).await
}

/// Add a tag to a link.
#[utoipa::path(
    put,
    path = "/{id}/tags/{tag}",
    tag = "links",
    params(("id" = Uuid, Path), ("tag" = String, Path)),
    security(("BearerAuth" = [])),
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
async fn insert_tag(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<(Uuid, String)>,
) -> Result<Json<Vec<String>>, Error> {
    super::tags::insert(state, kind, path).await
}

/// Remove a tag from a link.
#[utoipa::path(
    delete,
    path = "/{id}/tags/{tag}",
    tag = "links",
    params(("id" = Uuid, Path), ("tag" = String, Path)),
    security(("BearerAuth" = [])),
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
async fn remove_tag(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<(Uuid, String)>,
) -> Result<Json<Vec<String>>, Error> {
    super::tags::remove(state, kind, path).await
}

/// List logs for a link.
#[utoipa::path(
    get,
    path = "/{id}/logs",
    tag = "links",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
async fn list_logs(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<Uuid>,
) -> Result<Json<Vec<Log>>, Error> {
    super::logs::list(state, kind, path).await
}

/// Replace logs for a link.
#[utoipa::path(
    put,
    path = "/{id}/logs",
    tag = "links",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Vec<media::logs::Body>)),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
async fn set_logs(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<Uuid>,
    body: Json<Vec<media::logs::Body>>,
) -> Result<Json<Vec<Log>>, Error> {
    super::logs::set(state, kind, path, body).await
}

/// Add a log to a link.
#[utoipa::path(
    post,
    path = "/{id}/logs",
    tag = "links",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(media::logs::Body)),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
async fn insert_log(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<Uuid>,
    body: Json<media::logs::Body>,
) -> Result<Json<Vec<Log>>, Error> {
    super::logs::insert(state, kind, path, body).await
}

/// Remove a log from a link.
#[utoipa::path(
    delete,
    path = "/{id}/logs/{log}",
    tag = "links",
    params(("id" = Uuid, Path), ("log" = Uuid, Path)),
    security(("BearerAuth" = [])),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
async fn remove_log(
    state: State<Pool>,
    kind: Option<Extension<media::Kind>>,
    path: Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Log>>, Error> {
    super::logs::remove(state, kind, path).await
}
