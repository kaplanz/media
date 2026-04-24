//! Root listing route.

use std::collections::HashMap;
use std::fmt::Write as _;

use axum::extract::{Query, State};
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

use super::query::Order;
use crate::axum::extract::{Error, Json};

pub fn router() -> Router<SqlitePool> {
    Router::new().routes(routes!(list))
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

impl Sort {
    fn as_col(self) -> &'static str {
        match self {
            Self::Created => "media.created",
            Self::Updated => "media.updated",
        }
    }
}

/// Query parameters for listing all media.
#[derive(Clone, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::IntoParams)]
struct Params {
    /// Filter by tag.
    tag: Option<String>,
    /// Field to sort by.
    sort: Option<Sort>,
    /// Sort direction.
    order: Option<Order>,
    /// Maximum number of results.
    limit: Option<i64>,
    /// Number of results to skip.
    offset: Option<i64>,
}

#[derive(sqlx::FromRow)]
struct Row {
    id: Uuid,
    kind: String,
    created: i64,
    updated: i64,
    // book
    isbn: Option<String>,
    hcid: Option<i64>,
    book_title: Option<String>,
    cover: Option<String>,
    about: Option<String>,
    color: Option<String>,
    // film
    film_tmdb: Option<i64>,
    film_title: Option<String>,
    film_year: Option<i64>,
    film_rated: Option<i64>,
    // game
    tgdb: Option<i64>,
    game_title: Option<String>,
    system: Option<String>,
    owned: Option<i64>,
    game_rated: Option<i64>,
    // link
    url: Option<String>,
    link_title: Option<String>,
    // show
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

#[utoipa::path(get, path = "/", tag = "media",
    params(Params),
    responses((status = 200, body = Vec<Record>)))]
async fn list(
    State(db): State<SqlitePool>,
    Query(params): Query<Params>,
) -> Result<Json<Vec<Record>>, Error> {
    let sort_col = params.sort.unwrap_or_default().as_col();
    let order = params.order.unwrap_or_default().as_str();

    let mut sql = String::from(
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
         LEFT JOIN shows ON shows.id = media.id",
    );
    if params.tag.is_some() {
        sql.push_str(
            " WHERE EXISTS \
             (SELECT 1 FROM tags WHERE tags.media = media.id AND tags.label = ?)",
        );
    }
    write!(sql, " ORDER BY {sort_col} {order}").unwrap();
    if params.limit.is_some() {
        sql.push_str(" LIMIT ? OFFSET ?");
    }

    let mut stmt = sqlx::query_as::<_, Row>(&sql);
    if let Some(ref tag) = params.tag {
        stmt = stmt.bind(tag.as_str());
    }
    if let Some(limit) = params.limit {
        stmt = stmt.bind(limit).bind(params.offset.unwrap_or(0));
    }

    let rows = stmt
        .fetch_all(&db)
        .await
        .inspect_err(|err| tracing::error!("{err}"))
        .map_err(Error::from)?;

    let all_tags =
        sqlx::query_as::<_, (Uuid, String)>("SELECT media, label FROM tags ORDER BY label")
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
