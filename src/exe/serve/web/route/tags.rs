//! Tag management routes.

use std::collections::HashMap;

use axum::Extension;
use axum::extract::{Query, State};
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use media::book::Book;
use media::film::Film;
use media::game::Game;
use media::link::Link;
use media::show::Show;
use media::{Item, Kind, Meta, Record};
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::axum::extract::{Error, Json, Path};
use crate::db::{self, Conn, Pool, Uuid as DbUuid};
use crate::schema::{books, films, games, links, media as m, shows, tags};

type Response = Record<Item>;

#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
    /// Filter by media kind.
    #[param(inline)]
    kind: Option<Kind>,
}

pub fn router() -> Router<Pool> {
    Router::new()
        .routes(routes!(all))
        .routes(routes!(fetch))
        .routes(routes!(list, set))
        .routes(routes!(insert, remove))
}

/// List all distinct tags.
#[utoipa::path(
    get,
    path = "/tags",
    tag = "media",
    params(Params),
    responses((status = 200, body = Vec<String>)),
)]
async fn all(
    State(db): State<Pool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<String>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let labels: Vec<String> = if let Some(kind) = params.kind {
        tags::table
            .inner_join(m::table)
            .filter(m::kind.eq(kind.to_string()))
            .select(tags::label)
            .distinct()
            .order_by(tags::label)
            .load(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
    } else {
        tags::table
            .select(tags::label)
            .distinct()
            .order_by(tags::label)
            .load(&mut conn)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?
    };
    Ok(Json(labels))
}

/// Fetch media items by tag.
#[utoipa::path(
    get,
    path = "/tags/{tag}",
    tag = "media",
    params(("tag" = String, Path)),
    responses((status = 200)),
)]
#[allow(clippy::too_many_lines)]
async fn fetch(
    State(db): State<Pool>,
    Path(tag): Path<String>,
) -> Result<Json<Vec<Response>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;

    // Build query
    let rows: Vec<(DbUuid, String, i64, i64)> = m::table
        .filter(diesel::dsl::exists(
            tags::table
                .filter(tags::media.eq(m::id))
                .filter(tags::label.eq(&tag)),
        ))
        .select((m::id, m::kind, m::created, m::updated))
        .order_by(m::created.desc())
        .load(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

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
    let mut tags = load_tags_for(&mut conn, &ids).await?;

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

/// List tags for a media item.
#[utoipa::path(
    get,
    path = "/{id}/tags",
    tag = "media",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
pub(super) async fn list(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<String>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    let labels: Vec<String> = tags::table
        .filter(tags::media.eq(uid))
        .select(tags::label)
        .order_by(tags::label)
        .load(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(Json(labels))
}

/// Replace tags for a media item.
#[utoipa::path(
    put,
    path = "/{id}/tags",
    tag = "media",
    params(("id" = Uuid, Path)),
    security(("BearerAuth" = [])),
    request_body = Vec<String>,
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
pub(super) async fn set(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path(id): Path<Uuid>,
    Json(labels): Json<Vec<String>>,
) -> Result<Json<Vec<String>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    conn.transaction(|conn| {
        let labels = labels.clone();
        async move {
            diesel::delete(tags::table.filter(tags::media.eq(uid)))
                .execute(conn)
                .await?;
            for label in &labels {
                diesel::insert_into(tags::table)
                    .values((tags::media.eq(uid), tags::label.eq(label)))
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
    let labels: Vec<String> = tags::table
        .filter(tags::media.eq(uid))
        .select(tags::label)
        .order_by(tags::label)
        .load(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(Json(labels))
}

/// Add a tag to a media item.
#[utoipa::path(
    put,
    path = "/{id}/tags/{tag}",
    tag = "media",
    params(("id" = Uuid, Path), ("tag" = String, Path)),
    security(("BearerAuth" = [])),
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
pub(super) async fn insert(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path((id, tag)): Path<(Uuid, String)>,
) -> Result<Json<Vec<String>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    diesel::insert_into(tags::table)
        .values((tags::media.eq(uid), tags::label.eq(&tag)))
        .on_conflict_do_nothing()
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
    let labels: Vec<String> = tags::table
        .filter(tags::media.eq(uid))
        .select(tags::label)
        .order_by(tags::label)
        .load(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(Json(labels))
}

/// Remove a tag from a media item.
#[utoipa::path(
    delete,
    path = "/{id}/tags/{tag}",
    tag = "media",
    params(("id" = Uuid, Path), ("tag" = String, Path)),
    security(("BearerAuth" = [])),
    responses((status = 200, body = Vec<String>), (status = 404)),
)]
pub(super) async fn remove(
    State(db): State<Pool>,
    kind: Option<Extension<Kind>>,
    Path((id, label)): Path<(Uuid, String)>,
) -> Result<Json<Vec<String>>, Error> {
    let mut conn = db::get_conn(&db).await.map_err(Error::from)?;
    let uid = DbUuid::from(id);
    if !exists(&mut conn, uid, kind.map(|Extension(k)| k)).await? {
        return Err(Error::NotFound);
    }
    let n = diesel::delete(
        tags::table
            .filter(tags::media.eq(uid))
            .filter(tags::label.eq(&label)),
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
    let labels: Vec<String> = tags::table
        .filter(tags::media.eq(uid))
        .select(tags::label)
        .order_by(tags::label)
        .load(&mut conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(Json(labels))
}

/// Load tags for a set of media IDs, grouped by ID.
pub(super) async fn load_tags_for(
    conn: &mut Conn,
    ids: &[DbUuid],
) -> Result<HashMap<Uuid, Vec<String>>, Error> {
    let pairs: Vec<(DbUuid, String)> = tags::table
        .filter(tags::media.eq_any(ids))
        .select((tags::media, tags::label))
        .order_by(tags::label)
        .load(conn)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;

    let mut map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (uid, label) in pairs {
        map.entry(uid.into()).or_default().push(label);
    }
    Ok(map)
}

async fn exists(conn: &mut Conn, id: DbUuid, kind: Option<Kind>) -> Result<bool, Error> {
    if let Some(k) = kind {
        m::table
            .filter(m::id.eq(id))
            .filter(m::kind.eq(k.to_string()))
            .count()
            .get_result::<i64>(conn)
            .await
    } else {
        m::table
            .filter(m::id.eq(id))
            .count()
            .get_result::<i64>(conn)
            .await
    }
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(Error::from)
    .map(|n| n > 0)
}
