//! Root media routes.

use axum::extract::State;
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
    Router::new().routes(routes!(fetch))
}

#[derive(sqlx::FromRow)]
struct BookRow {
    isbn: Option<String>,
    hcid: Option<i64>,
    title: String,
    cover: Option<String>,
    about: Option<String>,
    color: Option<String>,
}

#[derive(sqlx::FromRow)]
struct FilmRow {
    tmdb: Option<i64>,
    title: String,
    year: Option<i64>,
    rated: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct GameRow {
    tgdb: Option<i64>,
    title: String,
    system: Option<String>,
    owned: i64,
    rated: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct LinkRow {
    url: String,
    title: Option<String>,
}

#[derive(sqlx::FromRow)]
struct ShowRow {
    tmdb: Option<i64>,
    title: String,
    year: Option<i64>,
    rated: Option<i64>,
}

#[utoipa::path(get, path = "/{id}", tag = "media",
    params(("id" = Uuid, Path)),
    responses((status = 200, body = Record), (status = 404)))]
#[expect(clippy::too_many_lines)]
async fn fetch(State(db): State<SqlitePool>, Path(id): Path<Uuid>) -> Result<Json<Record>, Error> {
    let (kind, created, updated) = sqlx::query_as::<_, (String, i64, i64)>(
        "SELECT kind, created, updated FROM media WHERE id = ?",
    )
    .bind(id)
    .fetch_optional(&db)
    .await
    .inspect_err(|err| tracing::error!("{err}"))
    .map_err(Error::from)?
    .ok_or(Error::NotFound)?;

    let item = match kind.as_str() {
        "book" => {
            let row = sqlx::query_as::<_, BookRow>(
                "SELECT isbn, hcid, title, cover, about, color FROM books WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&db)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
            Kind::Book(Book {
                id,
                isbn: row.isbn,
                hcid: row.hcid,
                title: row.title,
                cover: row.cover,
                about: row.about,
                color: row.color,
            })
        }
        "film" => {
            let row = sqlx::query_as::<_, FilmRow>(
                "SELECT tmdb, title, year, rated FROM films WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&db)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
            Kind::Film(Film {
                id,
                tmdb: row.tmdb,
                title: row.title,
                year: row.year,
                rated: row.rated,
            })
        }
        "game" => {
            let row = sqlx::query_as::<_, GameRow>(
                "SELECT tgdb, title, system, owned, rated FROM games WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&db)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
            Kind::Game(Game {
                id,
                tgdb: row.tgdb,
                title: row.title,
                system: row.system,
                owned: row.owned,
                rated: row.rated,
            })
        }
        "link" => {
            let row = sqlx::query_as::<_, LinkRow>("SELECT url, title FROM links WHERE id = ?")
                .bind(id)
                .fetch_one(&db)
                .await
                .inspect_err(|err| tracing::error!("{err}"))
                .map_err(Error::from)?;
            Kind::Link(Link {
                id,
                url: row.url,
                title: row.title,
            })
        }
        "show" => {
            let row = sqlx::query_as::<_, ShowRow>(
                "SELECT tmdb, title, year, rated FROM shows WHERE id = ?",
            )
            .bind(id)
            .fetch_one(&db)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;
            Kind::Show(Show {
                id,
                tmdb: row.tmdb,
                title: row.title,
                year: row.year,
                rated: row.rated,
            })
        }
        _ => return Err(Error::NotFound),
    };

    let tags =
        sqlx::query_scalar::<_, String>("SELECT label FROM tags WHERE media = ? ORDER BY label")
            .bind(id)
            .fetch_all(&db)
            .await
            .inspect_err(|err| tracing::error!("{err}"))
            .map_err(Error::from)?;

    Ok(Json(Record {
        item,
        meta: Meta { created, updated },
        tags,
    }))
}
