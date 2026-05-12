//! JSON load format.

use std::fs::File;
use std::io::{BufReader, Stdin};

use anyhow::Context;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use either::Either;
use media::{Item, Record};

use crate::db::{self, Pool, Uuid as DbUuid};
use crate::schema::{books, films, games, links, media as m, shows, tags};

#[allow(clippy::too_many_lines)]
pub async fn run(pool: &Pool, reader: BufReader<Either<File, Stdin>>) -> anyhow::Result<()> {
    let records: Vec<Record<Item>> =
        serde_json::from_reader(reader).context("failed to parse JSON")?;

    let mut conn = db::get_conn(pool)
        .await
        .context("failed to get connection")?;

    conn.transaction(|conn| {
        async move {
            for record in &records {
                let (kind, id) = match &record.item {
                    Item::Book(b) => ("book", b.id),
                    Item::Film(f) => ("film", f.id),
                    Item::Game(g) => ("game", g.id),
                    Item::Link(l) => ("link", l.id),
                    Item::Show(s) => ("show", s.id),
                };
                let uid = DbUuid::from(id);

                diesel::insert_into(m::table)
                    .values((
                        m::id.eq(uid),
                        m::kind.eq(kind),
                        m::created.eq(record.meta.created),
                        m::updated.eq(record.meta.updated),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;

                match &record.item {
                    Item::Book(b) => {
                        diesel::insert_into(books::table)
                            .values((
                                books::id.eq(uid),
                                books::isbn.eq(&b.isbn),
                                books::hcid.eq(b.hcid),
                                books::title.eq(&b.title),
                                books::cover.eq(&b.cover),
                                books::about.eq(&b.about),
                                books::color.eq(&b.color),
                            ))
                            .on_conflict_do_nothing()
                            .execute(conn)
                            .await?;
                    }
                    Item::Film(f) => {
                        diesel::insert_into(films::table)
                            .values((
                                films::id.eq(uid),
                                films::tmdb.eq(f.tmdb),
                                films::title.eq(&f.title),
                                films::year.eq(f.year),
                                films::rated.eq(f.rated),
                            ))
                            .on_conflict_do_nothing()
                            .execute(conn)
                            .await?;
                    }
                    Item::Game(g) => {
                        diesel::insert_into(games::table)
                            .values((
                                games::id.eq(uid),
                                games::title.eq(&g.title),
                                games::system.eq(&g.system),
                                games::rated.eq(g.rated),
                            ))
                            .on_conflict_do_nothing()
                            .execute(conn)
                            .await?;
                    }
                    Item::Link(l) => {
                        diesel::insert_into(links::table)
                            .values((
                                links::id.eq(uid),
                                links::url.eq(&l.url),
                                links::title.eq(&l.title),
                            ))
                            .on_conflict_do_nothing()
                            .execute(conn)
                            .await?;
                    }
                    Item::Show(s) => {
                        diesel::insert_into(shows::table)
                            .values((
                                shows::id.eq(uid),
                                shows::tmdb.eq(s.tmdb),
                                shows::title.eq(&s.title),
                                shows::year.eq(s.year),
                                shows::rated.eq(s.rated),
                            ))
                            .on_conflict_do_nothing()
                            .execute(conn)
                            .await?;
                    }
                }

                for label in &record.tags {
                    diesel::insert_into(tags::table)
                        .values((tags::media.eq(uid), tags::label.eq(label)))
                        .on_conflict_do_nothing()
                        .execute(conn)
                        .await?;
                }
            }
            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .context("failed to load records")?;

    Ok(())
}
