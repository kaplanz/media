//! JSON load format.

use std::fs::File;
use std::io::{BufReader, Stdin};

use anyhow::Context;
use diesel::prelude::*;
use diesel_async::scoped_futures::ScopedFutureExt;
use diesel_async::{AsyncConnection, RunQueryDsl};
use either::Either;
use media::Item;

use crate::db::{self, Pool, Uuid as DbUuid};
use crate::exe::dump::Dump;
use crate::schema::{
    books,
    films,
    games,
    games_extras,
    games_owned,
    games_system,
    links,
    media as m,
    shows,
    tags,
};

#[allow(clippy::too_many_lines)]
pub async fn run(pool: &Pool, reader: BufReader<Either<File, Stdin>>) -> anyhow::Result<()> {
    let dump: Dump = serde_json::from_reader(reader).context("failed to parse JSON")?;

    let mut conn = db::get_conn(pool)
        .await
        .context("failed to get connection")?;

    conn.transaction(|conn| {
        async move {
            for record in &dump.media {
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

            for s in &dump.games.system {
                diesel::insert_into(games_system::table)
                    .values((
                        games_system::id.eq(DbUuid::from(s.id)),
                        games_system::title.eq(&s.title),
                        games_system::system.eq(&s.system),
                        games_system::model.eq(&s.model),
                        games_system::revision.eq(&s.revision),
                        games_system::serial.eq(&s.serial),
                        games_system::variation.eq(&s.variation),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
            }

            for o in &dump.games.owned {
                diesel::insert_into(games_owned::table)
                    .values((
                        games_owned::id.eq(DbUuid::from(o.id)),
                        games_owned::game.eq(DbUuid::from(o.game)),
                        games_owned::system.eq(&o.system),
                        games_owned::model.eq(&o.model),
                        games_owned::revision.eq(&o.revision),
                        games_owned::serial.eq(&o.serial),
                        games_owned::cib.eq(o.cib),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
            }

            for e in &dump.games.extras {
                diesel::insert_into(games_extras::table)
                    .values((
                        games_extras::id.eq(DbUuid::from(e.id)),
                        games_extras::title.eq(&e.title),
                        games_extras::system.eq(&e.system),
                        games_extras::model.eq(&e.model),
                        games_extras::revision.eq(&e.revision),
                        games_extras::serial.eq(&e.serial),
                        games_extras::variation.eq(&e.variation),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;
            }

            Ok::<(), diesel::result::Error>(())
        }
        .scope_boxed()
    })
    .await
    .context("failed to load records")?;

    Ok(())
}
