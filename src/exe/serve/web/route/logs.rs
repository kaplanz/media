//! Log management routes.

use std::collections::HashMap;

use axum::Extension;
use axum::extract::State;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use media::Kind;
use media::logs::{Body, Log};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Conn, Pool, Uuid as DbUuid};
use crate::schema::{logs, media as m};

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(list, set, insert))
        .routes(routes!(remove))
}

/// List logs for a media item.
#[utoipa::path(
    get,
    path = "/{id}/logs",
    tag = "media",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
pub(super) async fn list(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Log>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !super::tags::exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    Ok(Json(load(&mut conn, uid).await?))
}

/// Replace logs for a media item.
#[utoipa::path(
    put,
    path = "/{id}/logs",
    tag = "media",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Vec<Body>)),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
pub(super) async fn set(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path(id): Path<Uuid>,
    Json(bodies): Json<Vec<Body>>,
) -> Result<Json<Vec<Log>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !super::tags::exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    conn.transaction(|conn| {
        async move {
            diesel::delete(logs::table.filter(logs::media.eq(uid)))
                .execute(conn)
                .await?;
            for body in bodies {
                diesel::insert_into(logs::table)
                    .values((
                        logs::id.eq(DbUuid::from(Uuid::new_v4())),
                        logs::media.eq(uid),
                        logs::kind.eq(body.kind),
                        logs::date.eq(body.date.unwrap_or_else(db::timestamp)),
                    ))
                    .execute(conn)
                    .await?;
            }
            diesel::update(m::table.filter(m::id.eq(uid)))
                .set(m::updated.eq(db::timestamp()))
                .execute(conn)
                .await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .inspect_err(|err: &diesel::result::Error| tracing::error!("{err}"))
    .map_err(Error::from)?;
    Ok(Json(load(&mut conn, uid).await?))
}

/// Add a log to a media item.
#[utoipa::path(
    post,
    path = "/{id}/logs",
    tag = "media",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Body)),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
pub(super) async fn insert(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path(id): Path<Uuid>,
    Json(body): Json<Body>,
) -> Result<Json<Vec<Log>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !super::tags::exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    diesel::insert_into(logs::table)
        .values((
            logs::id.eq(DbUuid::from(Uuid::new_v4())),
            logs::media.eq(uid),
            logs::kind.eq(body.kind),
            logs::date.eq(body.date.unwrap_or_else(db::timestamp)),
        ))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    diesel::update(m::table.filter(m::id.eq(uid)))
        .set(m::updated.eq(db::timestamp()))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(Json(load(&mut conn, uid).await?))
}

/// Remove a log from a media item.
#[utoipa::path(
    delete,
    path = "/{id}/logs/{log}",
    tag = "media",
    params(("id" = Uuid, Path), ("log" = Uuid, Path)),
    security(("BearerAuth" = [])),
    responses((status = 200, body = Vec<Log>), (status = 404)),
)]
pub(super) async fn remove(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path((id, log)): Path<(Uuid, Uuid)>,
) -> Result<Json<Vec<Log>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !super::tags::exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    let n = diesel::delete(
        logs::table
            .filter(logs::media.eq(uid))
            .filter(logs::id.eq(DbUuid::from(log))),
    )
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
    Ok(Json(load(&mut conn, uid).await?))
}

/// Load logs for a media item, ordered by date.
async fn load(conn: &mut Conn, id: DbUuid) -> Result<Vec<Log>, Error> {
    logs::table
        .filter(logs::media.eq(id))
        .select((logs::id, logs::kind, logs::date))
        .order_by(logs::date)
        .load(conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)
}

/// Load logs for a set of media IDs, grouped by ID.
pub(super) async fn load_logs_for(
    conn: &mut Conn,
    ids: &[DbUuid],
) -> Result<HashMap<Uuid, Vec<Log>>, Error> {
    let rows: Vec<(DbUuid, Log)> = logs::table
        .filter(logs::media.eq_any(ids))
        .select((logs::media, (logs::id, logs::kind, logs::date)))
        .order_by(logs::date)
        .load(conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;

    let mut map: HashMap<Uuid, Vec<Log>> = HashMap::new();
    for (uid, log) in rows {
        map.entry(uid.into()).or_default().push(log);
    }
    Ok(map)
}
