//! Film routes.

use axum::Extension;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use media::film::{Body, Film, Patch};
use media::{Item, Meta};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use super::query::Order;
use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Pool, Uuid as DbUuid};
use crate::schema::{films, media as m, tags};

type Record = media::Record<Item>;

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, modify, remove))
        .routes(routes!(list_tags, set_tags))
        .routes(routes!(insert_tag, remove_tag))
        .layer(Extension(media::Kind::Film))
}

/// Sort field for films.
#[derive(Clone, Copy, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
enum Sort {
    /// Sort by title.
    Title,
    /// Sort by release year.
    Year,
    /// Sort by rating.
    Rating,
    /// Sort by creation time.
    #[default]
    Created,
    /// Sort by last update time.
    Updated,
}

/// Query parameters for listing films.
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

/// List films.
#[utoipa::path(
    get,
    path = "/",
    tag = "films",
    params(Params),
    responses((status = 200, body = Vec<Record>)),
)]
async fn list(
    State(db): State<Pool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Record>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;

    // Build query
    let mut query = films::table
        .inner_join(m::table)
        .select((films::all_columns, m::created, m::updated))
        .into_boxed();

    // Apply filters
    if let Some(q) = params.q {
        query = query.filter(films::title.like(format!("%{q}%")));
    }
    if let Some(tag) = params.tag {
        query = query.filter(diesel::dsl::exists(
            tags::table
                .filter(tags::media.eq(films::id))
                .filter(tags::label.eq(tag)),
        ));
    }

    // Sort and paginate
    let rows: Vec<(Film, i64, i64)> = {
        let q = match (
            params.sort.unwrap_or_default(),
            params.order.unwrap_or_default(),
        ) {
            (Sort::Title, Order::Asc) => query.order_by(films::title.asc()),
            (Sort::Title, Order::Desc) => query.order_by(films::title.desc()),
            (Sort::Year, Order::Asc) => query.order_by(films::year.asc()),
            (Sort::Year, Order::Desc) => query.order_by(films::year.desc()),
            (Sort::Rating, Order::Asc) => query.order_by(films::rating.asc()),
            (Sort::Rating, Order::Desc) => query.order_by(films::rating.desc()),
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
    let ids: Vec<DbUuid> = rows.iter().map(|(f, _, _)| f.id.into()).collect();
    let mut tags = super::tags::load_tags_for(&mut conn, &ids).await?;

    let records = rows
        .into_iter()
        .map(|(film, created, updated)| {
            let tags = tags.remove(&film.id).unwrap_or_default();
            let item = Item::Film(film);
            Record {
                item,
                meta: Meta { created, updated },
                tags,
            }
        })
        .collect();

    Ok(Json(records))
}

/// Fetch a film by ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "films",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Record), (status = 404)),
)]
async fn fetch(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<Json<Record>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);

    // Load item
    let (film, created, updated) = films::table
        .inner_join(m::table)
        .select((films::all_columns, m::created, m::updated))
        .filter(films::id.eq(uid))
        .first::<(Film, i64, i64)>(&mut conn)
        .await
        .optional()
        .map_err(Error::from)?
        .ok_or(Error::NotFound)?;

    // Load tags
    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();

    let item = Item::Film(film);
    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        tags,
    }))
}

/// Create a film.
#[utoipa::path(
    post,
    path = "/",
    tag = "films",
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
                .values((m::id.eq(uid), m::kind.eq("film")))
                .execute(conn)
                .await?;
            diesel::insert_into(films::table)
                .values((
                    films::id.eq(uid),
                    films::tmdb.eq(body.tmdb),
                    films::title.eq(&body.title),
                    films::year.eq(body.year),
                    films::rating.eq(body.rating),
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

/// Update a film.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "films",
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
    let n = diesel::update(films::table.filter(films::id.eq(uid)))
        .set((
            films::tmdb.eq(body.tmdb),
            films::title.eq(&body.title),
            films::year.eq(body.year),
            films::rating.eq(body.rating),
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
    // Load updated item
    let (film, created, updated) = films::table
        .inner_join(m::table)
        .select((films::all_columns, m::created, m::updated))
        .filter(films::id.eq(uid))
        .first::<(Film, i64, i64)>(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    // Load tags
    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let item = Item::Film(film);
    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        tags,
    }))
}

/// Modify a film.
#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "films",
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
        let n = diesel::update(films::table.filter(films::id.eq(uid)))
            .set((
                body.tmdb.map(|v| films::tmdb.eq(v)),
                body.title.map(|v| films::title.eq(v)),
                body.year.map(|v| films::year.eq(v)),
                body.rating.map(|v| films::rating.eq(v)),
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
    let (film, created, updated) = films::table
        .inner_join(m::table)
        .select((films::all_columns, m::created, m::updated))
        .filter(films::id.eq(uid))
        .first::<(Film, i64, i64)>(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    // Load tags
    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    let item = Item::Film(film);
    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        tags,
    }))
}

/// Delete a film.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "films",
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

/// List tags for a film.
#[utoipa::path(
    get,
    path = "/{id}/tags",
    tag = "films",
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

/// Replace tags for a film.
#[utoipa::path(
    put,
    path = "/{id}/tags",
    tag = "films",
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

/// Add a tag to a film.
#[utoipa::path(
    put,
    path = "/{id}/tags/{tag}",
    tag = "films",
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

/// Remove a tag from a film.
#[utoipa::path(
    delete,
    path = "/{id}/tags/{tag}",
    tag = "films",
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
