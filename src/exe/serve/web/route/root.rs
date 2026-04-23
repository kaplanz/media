//! Root media routes.

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use media::book::Book;
use media::film::Film;
use media::game::Game;
use media::link::Link;
use media::show::Show;
use media::{Item, Meta, Record};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use super::query::Order;
use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Pool, Uuid as DbUuid};
use crate::schema::{books, films, games, links, media as m, shows, tags};

type Response = Record<Item>;

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(list))
        .routes(routes!(fetch, remove))
}

/// Sort field for all media.
#[derive(Clone, Copy, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
enum Sort {
    /// Sort by creation time.
    #[default]
    Created,
    /// Sort by last update time.
    Updated,
}

/// Query parameters for listing all media.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
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

/// List all media.
#[utoipa::path(
    get,
    path = "/",
    tag = "media",
    params(Params),
    responses((status = 200)),
)]
#[allow(clippy::too_many_lines)]
async fn list(
    State(db): State<Pool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Response>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;

    // Build query
    let mut query = m::table
        .select((m::id, m::kind, m::created, m::updated))
        .into_boxed();

    // Apply filters
    if let Some(tag) = params.tag {
        query = query.filter(diesel::dsl::exists(
            tags::table
                .filter(tags::media.eq(m::id))
                .filter(tags::label.eq(tag)),
        ));
    }

    // Sort and paginate
    let rows: Vec<(DbUuid, String, i64, i64)> = {
        let q = match (
            params.sort.unwrap_or_default(),
            params.order.unwrap_or_default(),
        ) {
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

    let ids: Vec<DbUuid> = rows.iter().map(|r| r.0).collect();

    // Partition by kind
    let mut book = Vec::new();
    let mut film = Vec::new();
    let mut game = Vec::new();
    let mut link = Vec::new();
    let mut show = Vec::new();

    for (uid, kind, _, _) in &rows {
        match kind.as_str() {
            "book" => book.push(*uid),
            "film" => film.push(*uid),
            "game" => game.push(*uid),
            "link" => link.push(*uid),
            "show" => show.push(*uid),
            _ => {}
        }
    }

    // Load items
    let mut items: HashMap<Uuid, Item> = HashMap::new();

    if !book.is_empty() {
        for b in books::table
            .filter(books::id.eq_any(&book))
            .select(books::all_columns)
            .load::<Book>(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
        {
            items.insert(b.id, Item::Book(b));
        }
    }
    if !film.is_empty() {
        for f in films::table
            .filter(films::id.eq_any(&film))
            .select(films::all_columns)
            .load::<Film>(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
        {
            items.insert(f.id, Item::Film(f));
        }
    }
    if !game.is_empty() {
        for g in games::table
            .filter(games::id.eq_any(&game))
            .select(games::all_columns)
            .load::<Game>(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
        {
            items.insert(g.id, Item::Game(g));
        }
    }
    if !link.is_empty() {
        for l in links::table
            .filter(links::id.eq_any(&link))
            .select(links::all_columns)
            .load::<Link>(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
        {
            items.insert(l.id, Item::Link(l));
        }
    }
    if !show.is_empty() {
        for s in shows::table
            .filter(shows::id.eq_any(&show))
            .select(shows::all_columns)
            .load::<Show>(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
        {
            items.insert(s.id, Item::Show(s));
        }
    }

    // Load tags
    let mut tags = super::tags::load_tags_for(&mut conn, &ids).await?;

    let records = rows
        .into_iter()
        .filter_map(|(uid, _, created, updated)| {
            let id: Uuid = uid.into();
            let item = items.remove(&id)?;
            let tags = tags.remove(&id).unwrap_or_default();
            Some(Record {
                item,
                meta: Meta { created, updated },
                tags,
            })
        })
        .collect();

    Ok(Json(records))
}

/// Fetch any media item by ID.
#[utoipa::path(
    get,
    path = "/{id}",
    tag = "media",
    params(("id" = Uuid, Path)),
    responses((status = 200), (status = 404)),
)]
async fn fetch(State(db): State<Pool>, Path(id): Path<Uuid>) -> Result<Json<Response>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);

    let (kind, created, updated) = m::table
        .filter(m::id.eq(uid))
        .select((m::kind, m::created, m::updated))
        .first::<(String, i64, i64)>(&mut conn)
        .await
        .optional()
        .map_err(Error::from)?
        .ok_or(Error::NotFound)?;

    let item = match kind.as_str() {
        "book" => {
            let book = books::table
                .filter(books::id.eq(uid))
                .select(books::all_columns)
                .first::<Book>(&mut conn)
                .await
                .map_err(Error::from)?;
            Item::Book(book)
        }
        "film" => {
            let film = films::table
                .filter(films::id.eq(uid))
                .select(films::all_columns)
                .first::<Film>(&mut conn)
                .await
                .map_err(Error::from)?;
            Item::Film(film)
        }
        "game" => {
            let game = games::table
                .filter(games::id.eq(uid))
                .select(games::all_columns)
                .first::<Game>(&mut conn)
                .await
                .map_err(Error::from)?;
            Item::Game(game)
        }
        "link" => {
            let link = links::table
                .filter(links::id.eq(uid))
                .select(links::all_columns)
                .first::<Link>(&mut conn)
                .await
                .map_err(Error::from)?;
            Item::Link(link)
        }
        "show" => {
            let show = shows::table
                .filter(shows::id.eq(uid))
                .select(shows::all_columns)
                .first::<Show>(&mut conn)
                .await
                .map_err(Error::from)?;
            Item::Show(show)
        }
        _ => return Err(Error::NotFound),
    };

    let tags = super::tags::load_tags_for(&mut conn, &[uid])
        .await?
        .remove(&id)
        .unwrap_or_default();

    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        tags,
    }))
}

/// Delete any media item by ID.
#[utoipa::path(
    delete,
    path = "/{id}",
    tag = "media",
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
