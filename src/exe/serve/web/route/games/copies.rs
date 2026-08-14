//! Owned game copy routes.

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use media::game::Game;
use media::game::copies::{Body, Copies, Patch, Row};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use super::super::query::Order;
use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Conn, Pool, Uuid as DbUuid};
use crate::schema::{games as g, games_copies as t, games_copies_ref as w};

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(list, create))
        .routes(routes!(fetch, update, modify, remove))
}

/// Load the games of each copy, keyed by copy ID.
async fn load_games_for(
    conn: &mut Conn,
    ids: &[DbUuid],
) -> Result<HashMap<Uuid, Vec<Game>>, Error> {
    let rows: Vec<(DbUuid, Game)> = w::table
        .inner_join(g::table.on(g::id.eq(w::game)))
        .filter(w::copy.eq_any(ids))
        .select((w::copy, g::all_columns))
        .order_by((w::copy, w::idx))
        .load(conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    let mut games: HashMap<Uuid, Vec<Game>> = HashMap::new();
    for (copy, game) in rows {
        games.entry(copy.into()).or_default().push(game);
    }
    Ok(games)
}

/// Replace the games of a copy.
async fn set_games(conn: &mut Conn, copy: DbUuid, games: &[Uuid]) -> QueryResult<()> {
    diesel::delete(w::table.filter(w::copy.eq(copy)))
        .execute(conn)
        .await?;
    for (idx, game) in games.iter().enumerate() {
        diesel::insert_into(w::table)
            .values((
                w::copy.eq(copy),
                w::game.eq(DbUuid::from(*game)),
                w::idx.eq(i64::try_from(idx).unwrap_or(i64::MAX)),
            ))
            .execute(conn)
            .await?;
    }
    Ok(())
}

/// Sort field for owned game copies.
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
    /// Sort by title.
    Title,
}

/// Query parameters for listing owned game copies.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
    /// Filter by included game ID.
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

/// List owned game copies.
#[utoipa::path(
    get,
    path = "/",
    tag = "games/copies",
    params(Params),
    responses((status = 200, body = Vec<Copies>)),
)]
async fn list(
    State(db): State<Pool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Copies>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;

    let mut query = t::table.select(t::all_columns).into_boxed();

    if let Some(game) = params.game {
        query = query.filter(
            t::id.eq_any(
                w::table
                    .filter(w::game.eq(DbUuid::from(game)))
                    .select(w::copy),
            ),
        );
    }
    if let Some(system) = params.system {
        query = query.filter(t::system.eq(system));
    }

    let rows: Vec<Row> = {
        let q = match (
            params.sort.unwrap_or_default(),
            params.order.unwrap_or_default(),
        ) {
            (Sort::System, Order::Asc) => query.order_by(t::system.asc()),
            (Sort::System, Order::Desc) => query.order_by(t::system.desc()),
            (Sort::Model, Order::Asc) => query.order_by(t::model.asc()),
            (Sort::Model, Order::Desc) => query.order_by(t::model.desc()),
            (Sort::Title, Order::Asc) => query.order_by(t::title.asc()),
            (Sort::Title, Order::Desc) => query.order_by(t::title.desc()),
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

    // Resolve games
    let ids: Vec<DbUuid> = rows.iter().map(|row| DbUuid::from(row.id)).collect();
    let mut games = load_games_for(&mut conn, &ids).await?;
    let rows = rows
        .into_iter()
        .map(|row| {
            let games = games.remove(&row.id).unwrap_or_default();
            row.resolve(games)
        })
        .collect();

    Ok(Json(rows))
}

/// Fetch an owned game copy by ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "games/copies",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Copies), (status = 404)),
)]
async fn fetch(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<Json<Copies>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    let row = t::table
        .select(t::all_columns)
        .filter(t::id.eq(uid))
        .first::<Row>(&mut conn)
        .await
        .optional()
        .map_err(Error::from)?
        .ok_or(Error::NotFound)?;
    let games = load_games_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    Ok(Json(row.resolve(games)))
}

/// Create an owned game copy.
#[utoipa::path(
    post,
    path = "/",
    tag = "games/copies",
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
    conn.transaction(|conn| {
        async move {
            diesel::insert_into(t::table)
                .values((
                    t::id.eq(uid),
                    t::title.eq(&body.title),
                    t::system.eq(&body.system),
                    t::region.eq(&body.region),
                    t::model.eq(&body.model),
                    t::revision.eq(&body.revision),
                    t::serial.eq(&body.serial),
                    t::complete.eq(body.complete.unwrap_or(false)),
                    t::modified.eq(body.modified.unwrap_or(false)),
                ))
                .execute(conn)
                .await?;
            set_games(conn, uid, &body.game).await?;
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .inspect_err(|err: &diesel::result::Error| tracing::error!("{err}"))
    .map_err(Error::from)?;
    Ok((StatusCode::CREATED, Json(id)))
}

/// Update an owned game copy.
#[utoipa::path(
    put,
    path = "/{id}",
    tag = "games/copies",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Body)),
    responses((status = 200, body = Copies), (status = 404)),
)]
async fn update(
    State(db): State<Pool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Body>,
) -> Result<Json<Copies>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    let n = conn
        .transaction(|conn| {
            async move {
                let n = diesel::update(t::table.filter(t::id.eq(uid)))
                    .set((
                        t::title.eq(&body.title),
                        t::system.eq(&body.system),
                        t::region.eq(&body.region),
                        t::model.eq(&body.model),
                        t::revision.eq(&body.revision),
                        t::serial.eq(&body.serial),
                        t::complete.eq(body.complete.unwrap_or(false)),
                        t::modified.eq(body.modified.unwrap_or(false)),
                    ))
                    .execute(conn)
                    .await?;
                if n > 0 {
                    set_games(conn, uid, &body.game).await?;
                }
                Ok::<usize, diesel::result::Error>(n)
            }
            .scope_boxed()
        })
        .await
        .inspect_err(|err: &diesel::result::Error| tracing::error!("{err}"))
        .map_err(Error::from)?;
    if n == 0 {
        return Err(Error::NotFound);
    }
    let row = t::table
        .select(t::all_columns)
        .filter(t::id.eq(uid))
        .first::<Row>(&mut conn)
        .await
        .map_err(Error::from)?;
    let games = load_games_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    Ok(Json(row.resolve(games)))
}

/// Modify an owned game copy.
#[utoipa::path(
    patch,
    path = "/{id}",
    tag = "games/copies",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body(content = inline(Patch)),
    responses((status = 200, body = Copies), (status = 404)),
)]
async fn modify(
    State(db): State<Pool>,
    Path(id): Path<Uuid>,
    Json(body): Json<Patch>,
) -> Result<Json<Copies>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    // Apply present fields
    if !body.is_empty() {
        conn.transaction(|conn| {
            async move {
                if body.has_columns() {
                    diesel::update(t::table.filter(t::id.eq(uid)))
                        .set((
                            body.title.map(|v| t::title.eq(v)),
                            body.system.map(|v| t::system.eq(v)),
                            body.region.map(|v| t::region.eq(v)),
                            body.model.map(|v| t::model.eq(v)),
                            body.revision.map(|v| t::revision.eq(v)),
                            body.serial.map(|v| t::serial.eq(v)),
                            body.complete.map(|v| t::complete.eq(v)),
                            body.modified.map(|v| t::modified.eq(v)),
                        ))
                        .execute(conn)
                        .await?;
                }
                if let Some(game) = &body.game {
                    set_games(conn, uid, game).await?;
                }
                Ok::<(), diesel::result::Error>(())
            }
            .scope_boxed()
        })
        .await
        .inspect_err(|err: &diesel::result::Error| tracing::error!("{err}"))
        .map_err(Error::from)?;
    }
    let row = t::table
        .select(t::all_columns)
        .filter(t::id.eq(uid))
        .first::<Row>(&mut conn)
        .await
        .optional()
        .map_err(Error::from)?
        .ok_or(Error::NotFound)?;
    let games = load_games_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();
    Ok(Json(row.resolve(games)))
}

/// Delete an owned game copy.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "games/copies",
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
