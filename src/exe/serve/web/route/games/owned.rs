//! Owned game release routes.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use media::game::owned::{Body, Owned, Patch};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use super::super::query::Order;
use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Pool, Uuid as DbUuid};
use crate::schema::games_owned as t;

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, modify, remove))
}

/// Sort field for owned game releases.
#[derive(Clone, Copy, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
enum Sort {
    /// Sort by platform.
    #[default]
    System,
    /// Sort by model.
    Model,
}

/// Query parameters for listing owned game releases.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
    /// Filter by game ID.
    game: Option<Uuid>,
    /// Filter by platform.
    system: Option<String>,
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

/// List owned game releases.
#[utoipa::path(
    get,
    path = "/",
    tag = "games/owned",
    params(Params),
    responses((status = 200, body = Vec<Owned>)),
)]
async fn list(
    State(db): State<Pool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Owned>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;

    let mut query = t::table.select(t::all_columns).into_boxed();

    if let Some(game) = params.game {
        query = query.filter(t::game.eq(DbUuid::from(game)));
    }
    if let Some(system) = params.system {
        query = query.filter(t::system.eq(system));
    }

    let rows: Vec<Owned> = {
        let q = match (
            params.sort.unwrap_or_default(),
            params.order.unwrap_or_default(),
        ) {
            (Sort::System, Order::Asc) => query.order_by(t::system.asc()),
            (Sort::System, Order::Desc) => query.order_by(t::system.desc()),
            (Sort::Model, Order::Asc) => query.order_by(t::model.asc()),
            (Sort::Model, Order::Desc) => query.order_by(t::model.desc()),
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

    Ok(Json(rows))
}

/// Fetch an owned game release by ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "games/owned",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Owned), (status = 404)),
)]
async fn fetch(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<Json<Owned>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let row = t::table
        .select(t::all_columns)
        .filter(t::id.eq(DbUuid::from(id)))
        .first::<Owned>(&mut conn)
        .await
        .optional()
        .map_err(Error::from)?
        .ok_or(Error::NotFound)?;
    Ok(Json(row))
}

/// Create an owned game release.
#[utoipa::path(
    post,
    path = "/",
    tag = "games/owned",
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
    diesel::insert_into(t::table)
        .values((
            t::id.eq(DbUuid::from(id)),
            t::game.eq(DbUuid::from(body.game)),
            t::system.eq(&body.system),
            t::region.eq(&body.region),
            t::model.eq(&body.model),
            t::revision.eq(&body.revision),
            t::serial.eq(&body.serial),
            t::complete.eq(body.complete.unwrap_or(false)),
            t::modified.eq(body.modified.unwrap_or(false)),
        ))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok((StatusCode::CREATED, Json(id)))
}

/// Update an owned game release.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "games/owned",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Body)),
    responses((status = 200, body = Owned), (status = 404)),
)]
async fn update(
    State(db): State<Pool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Body>,
) -> Result<Json<Owned>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    let n = diesel::update(t::table.filter(t::id.eq(uid)))
        .set((
            t::game.eq(DbUuid::from(body.game)),
            t::system.eq(&body.system),
            t::region.eq(&body.region),
            t::model.eq(&body.model),
            t::revision.eq(&body.revision),
            t::serial.eq(&body.serial),
            t::complete.eq(body.complete.unwrap_or(false)),
            t::modified.eq(body.modified.unwrap_or(false)),
        ))
        .execute(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    if n == 0 {
        return Err(Error::NotFound);
    }
    let row = t::table
        .select(t::all_columns)
        .filter(t::id.eq(uid))
        .first::<Owned>(&mut conn)
        .await
        .map_err(Error::from)?;
    Ok(Json(row))
}

/// Modify an owned game release.
#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "games/owned",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Patch)),
    responses((status = 200, body = Owned), (status = 404)),
)]
async fn modify(
    State(db): State<Pool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Patch>,
) -> Result<Json<Owned>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    // Apply present fields
    if !body.is_empty() {
        let n = diesel::update(t::table.filter(t::id.eq(uid)))
            .set((
                body.game.map(|v| t::game.eq(DbUuid::from(v))),
                body.system.map(|v| t::system.eq(v)),
                body.region.map(|v| t::region.eq(v)),
                body.model.map(|v| t::model.eq(v)),
                body.revision.map(|v| t::revision.eq(v)),
                body.serial.map(|v| t::serial.eq(v)),
                body.complete.map(|v| t::complete.eq(v)),
                body.modified.map(|v| t::modified.eq(v)),
            ))
            .execute(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
        if n == 0 {
            return Err(Error::NotFound);
        }
    }
    let row = t::table
        .select(t::all_columns)
        .filter(t::id.eq(uid))
        .first::<Owned>(&mut conn)
        .await
        .map_err(Error::from)?;
    Ok(Json(row))
}

/// Delete an owned game release.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "games/owned",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    responses((status = 204), (status = 404)),
)]
async fn remove(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<StatusCode, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let n = diesel::delete(t::table.filter(t::id.eq(DbUuid::from(id))))
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
