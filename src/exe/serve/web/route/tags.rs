//! Tag management routes.

use std::collections::HashMap;

use axum::extract::State;
use axum::http::StatusCode;
use media::kind::book::Book;
use media::kind::film::Film;
use media::kind::game::Game;
use media::kind::link::Link;
use media::kind::show::Show;
use media::kind::{Kind, Meta, Record};
use sqlx::SqlitePool;
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_axum::routes;
use uuid::Uuid;

use crate::axum::extract::{Error, Json, Path};

pub fn router() -> Router<SqlitePool> {
    Router::new()
        .routes(routes!(all))
        .routes(routes!(fetch))
        .routes(routes!(list, set, add))
        .routes(routes!(remove))
}

#[utoipa::path(get, path = "/tags", tag = "media",
    responses((status = 200, body = Vec<String>)))]
async fn all(State(db): State<SqlitePool>) -> Result<Json<Vec<String>>, Error> {
    sqlx::query_scalar::<_, String>("SELECT DISTINCT label FROM tags ORDER BY label")
        .fetch_all(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)
        .map(Json)
}

#[utoipa::path(get, path = "/tags/{tag}", tag = "media",
    params(("tag" = String, Path)),
    responses((status = 200, body = Vec<Record>)))]
async fn fetch(
    State(db): State<SqlitePool>,
    Path(tag): Path<String>,
) -> Result<Json<Vec<Record>>, Error> {
    let rows = sqlx::query_as::<_, Row>(
        "SELECT media.id, media.kind, media.created, media.updated, \
         books.isbn, books.hcid, books.title AS book_title, \
         books.cover, books.about, books.color, \
         films.tmdb AS film_tmdb, films.title AS film_title, \
         films.year AS film_year, films.rated AS film_rated, \
         games.tgdb, games.title AS game_title, \
         games.system, games.owned, games.rated AS game_rated, \
         links.url, links.title AS link_title, \
         shows.tmdb AS show_tmdb, shows.title AS show_title, \
         shows.year AS show_year, shows.rated AS show_rated \
         FROM media \
         LEFT JOIN books ON books.id = media.id \
         LEFT JOIN films ON films.id = media.id \
         LEFT JOIN games ON games.id = media.id \
         LEFT JOIN links ON links.id = media.id \
         LEFT JOIN shows ON shows.id = media.id \
         WHERE EXISTS \
         (SELECT 1 FROM tags WHERE tags.media = media.id AND tags.label = ?) \
         ORDER BY media.created DESC",
    )
    .bind(&tag)
    .fetch_all(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(Error::from)?;

    if rows.is_empty() {
        return Err(Error::NotFound);
    }

    let all_tags = sqlx::query_as::<_, (Uuid, String)>(
        "SELECT media, label FROM tags \
         WHERE media IN (SELECT media FROM tags WHERE label = ?) \
         ORDER BY label",
    )
    .bind(&tag)
    .fetch_all(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(Error::from)?;

    let mut tag_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, label) in all_tags {
        tag_map.entry(id).or_default().push(label);
    }

    let records = rows
        .into_iter()
        .filter_map(|row| {
            let tags = tag_map.remove(&row.id).unwrap_or_default();
            row.into_record(tags)
        })
        .collect();

    Ok(Json(records))
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    kind: String,
    created: i64,
    updated: i64,
    isbn: Option<String>,
    hcid: Option<i64>,
    book_title: Option<String>,
    cover: Option<String>,
    about: Option<String>,
    color: Option<String>,
    film_tmdb: Option<i64>,
    film_title: Option<String>,
    film_year: Option<i64>,
    film_rated: Option<i64>,
    tgdb: Option<i64>,
    game_title: Option<String>,
    system: Option<String>,
    owned: Option<i64>,
    game_rated: Option<i64>,
    url: Option<String>,
    link_title: Option<String>,
    show_tmdb: Option<i64>,
    show_title: Option<String>,
    show_year: Option<i64>,
    show_rated: Option<i64>,
}

impl Row {
    fn into_record(self, tags: Vec<String>) -> Option<Record> {
        let item = match self.kind.as_str() {
            "book" => Kind::Book(Book {
                id: self.id,
                isbn: self.isbn,
                hcid: self.hcid,
                title: self.book_title?,
                cover: self.cover,
                about: self.about,
                color: self.color,
            }),
            "film" => Kind::Film(Film {
                id: self.id,
                tmdb: self.film_tmdb,
                title: self.film_title?,
                year: self.film_year,
                rated: self.film_rated,
            }),
            "game" => Kind::Game(Game {
                id: self.id,
                tgdb: self.tgdb,
                title: self.game_title?,
                system: self.system,
                owned: self.owned?,
                rated: self.game_rated,
            }),
            "link" => Kind::Link(Link {
                id: self.id,
                url: self.url?,
                title: self.link_title,
            }),
            "show" => Kind::Show(Show {
                id: self.id,
                tmdb: self.show_tmdb,
                title: self.show_title?,
                year: self.show_year,
                rated: self.show_rated,
            }),
            _ => return None,
        };
        Some(Record {
            item,
            meta: Meta {
                created: self.created,
                updated: self.updated,
            },
            tags,
        })
    }
}

async fn exists(db: &SqlitePool, id: Uuid) -> Result<bool, Error> {
    sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM media WHERE id = ?)")
        .bind(id)
        .fetch_one(db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)
}

#[utoipa::path(get, path = "/{id}/tags", tag = "media",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Vec<String>), (status = 404)))]
async fn list(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<String>>, Error> {
    if !exists(&db, id).await? {
        return Err(Error::NotFound);
    }
    sqlx::query_scalar::<_, String>("SELECT label FROM tags WHERE media = ? ORDER BY label")
        .bind(id)
        .fetch_all(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)
        .map(Json)
}

#[utoipa::path(put, path = "/{id}/tags", tag = "media",
    params(("id" = Uuid, Path)),
    request_body = Vec<String>,
    responses((status = 204), (status = 404)))]
async fn set(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
    Json(labels): Json<Vec<String>>,
) -> Result<StatusCode, Error> {
    if !exists(&db, id).await? {
        return Err(Error::NotFound);
    }
    let mut tx = db
        .begin()
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    sqlx::query("DELETE FROM tags WHERE media = ?")
        .bind(id)
        .execute(&mut *tx)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    for label in &labels {
        sqlx::query("INSERT INTO tags (media, label) VALUES (?, ?)")
            .bind(id)
            .bind(label)
            .execute(&mut *tx)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
    }
    tx.commit()
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/{id}/tags", tag = "media",
    params(("id" = Uuid, Path)),
    request_body = String,
    responses((status = 204), (status = 404)))]
async fn add(
    State(db): State<SqlitePool>,
    Path(id): Path<Uuid>,
    Json(label): Json<String>,
) -> Result<StatusCode, Error> {
    if !exists(&db, id).await? {
        return Err(Error::NotFound);
    }
    sqlx::query("INSERT OR IGNORE INTO tags (media, label) VALUES (?, ?)")
        .bind(id)
        .bind(&label)
        .execute(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/{id}/tags/{tag}", tag = "media",
    params(("id" = Uuid, Path), ("tag" = String, Path)),
    responses((status = 204), (status = 404)))]
async fn remove(
    State(db): State<SqlitePool>,
    Path((id, label)): Path<(Uuid, String)>,
) -> Result<StatusCode, Error> {
    sqlx::query("DELETE FROM tags WHERE media = ? AND label = ?")
        .bind(id)
        .bind(&label)
        .execute(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)
        .and_then(|res| {
            if res.rows_affected() > 0 {
                Ok(StatusCode::NO_CONTENT)
            } else {
                Err(Error::NotFound)
            }
        })
}
