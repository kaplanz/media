//! JSON dump format.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Stdout, Write};

use anyhow::Context;
use either::Either;
use media::kind::book::Book;
use media::kind::film::Film;
use media::kind::game::Game;
use media::kind::link::Link;
use media::kind::show::Show;
use media::kind::{Kind, Meta, Record};
use sqlx::SqlitePool;
use uuid::Uuid;

#[expect(clippy::too_many_lines)]
pub async fn run(
    pool: &SqlitePool,
    mut out: BufWriter<Either<File, Stdout>>,
) -> anyhow::Result<()> {
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
         ORDER BY media.created ASC",
    )
    .fetch_all(pool)
    .await
    .context("failed to query media")?;

    let all_tags =
        sqlx::query_as::<_, (Uuid, String)>("SELECT media, label FROM tags ORDER BY media, label")
            .fetch_all(pool)
            .await
            .context("failed to query tags")?;

    let mut tag_map: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (id, label) in all_tags {
        tag_map.entry(id).or_default().push(label);
    }

    let records: Vec<Record> = rows
        .into_iter()
        .filter_map(|r| {
            let tags = tag_map.remove(&r.id).unwrap_or_default();
            let item = match r.kind.as_str() {
                "book" => Kind::Book(Book {
                    id: r.id,
                    isbn: r.isbn,
                    hcid: r.hcid,
                    title: r.book_title?,
                    cover: r.cover,
                    about: r.about,
                    color: r.color,
                }),
                "film" => Kind::Film(Film {
                    id: r.id,
                    tmdb: r.film_tmdb,
                    title: r.film_title?,
                    year: r.film_year,
                    rated: r.film_rated,
                }),
                "game" => Kind::Game(Game {
                    id: r.id,
                    tgdb: r.tgdb,
                    title: r.game_title?,
                    system: r.system,
                    owned: r.owned?,
                    rated: r.game_rated,
                }),
                "link" => Kind::Link(Link {
                    id: r.id,
                    url: r.url?,
                    title: r.link_title,
                }),
                "show" => Kind::Show(Show {
                    id: r.id,
                    tmdb: r.show_tmdb,
                    title: r.show_title?,
                    year: r.show_year,
                    rated: r.show_rated,
                }),
                _ => return None,
            };
            Some(Record {
                item,
                meta: Meta {
                    created: r.created,
                    updated: r.updated,
                },
                tags,
            })
        })
        .collect();

    serde_json::to_writer_pretty(&mut out, &records).context("failed to serialize")?;
    writeln!(out)?;
    Ok(())
}
