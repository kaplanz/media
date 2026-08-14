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
    games_copies,
    games_copies_ref,
    games_extras,
    games_extras_ref,
    games_systems,
    games_systems_ref,
    links,
    logs,
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
                                films::rating.eq(f.rating),
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
                                games::rating.eq(g.rating),
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
                                shows::rating.eq(s.rating),
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

                for log in &record.logs {
                    diesel::insert_into(logs::table)
                        .values((
                            logs::id.eq(DbUuid::from(log.id)),
                            logs::media.eq(uid),
                            logs::kind.eq(log.kind),
                            logs::date.eq(log.date),
                        ))
                        .on_conflict_do_nothing()
                        .execute(conn)
                        .await?;
                }
            }

            for s in &dump.games.systems {
                let uid = DbUuid::from(s.row.id);
                diesel::insert_into(games_systems::table)
                    .values((
                        games_systems::id.eq(uid),
                        games_systems::title.eq(&s.row.title),
                        games_systems::system.eq(&s.row.system),
                        games_systems::region.eq(&s.row.region),
                        games_systems::model.eq(&s.row.model),
                        games_systems::revision.eq(&s.row.revision),
                        games_systems::serial.eq(&s.row.serial),
                        games_systems::variant.eq(&s.row.variant),
                        games_systems::complete.eq(s.row.complete),
                        games_systems::modified.eq(s.row.modified),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;

                for (idx, game) in s.game.iter().enumerate() {
                    diesel::insert_into(games_systems_ref::table)
                        .values((
                            games_systems_ref::system.eq(uid),
                            games_systems_ref::game.eq(DbUuid::from(*game)),
                            games_systems_ref::idx.eq(i64::try_from(idx).unwrap_or(i64::MAX)),
                        ))
                        .on_conflict_do_nothing()
                        .execute(conn)
                        .await?;
                }
            }

            for c in &dump.games.copies {
                let uid = DbUuid::from(c.row.id);
                diesel::insert_into(games_copies::table)
                    .values((
                        games_copies::id.eq(uid),
                        games_copies::title.eq(&c.row.title),
                        games_copies::system.eq(&c.row.system),
                        games_copies::region.eq(&c.row.region),
                        games_copies::model.eq(&c.row.model),
                        games_copies::revision.eq(&c.row.revision),
                        games_copies::serial.eq(&c.row.serial),
                        games_copies::complete.eq(c.row.complete),
                        games_copies::modified.eq(c.row.modified),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;

                for (idx, game) in c.game.iter().enumerate() {
                    diesel::insert_into(games_copies_ref::table)
                        .values((
                            games_copies_ref::copy.eq(uid),
                            games_copies_ref::game.eq(DbUuid::from(*game)),
                            games_copies_ref::idx.eq(i64::try_from(idx).unwrap_or(i64::MAX)),
                        ))
                        .on_conflict_do_nothing()
                        .execute(conn)
                        .await?;
                }
            }

            for e in &dump.games.extras {
                let uid = DbUuid::from(e.row.id);
                diesel::insert_into(games_extras::table)
                    .values((
                        games_extras::id.eq(uid),
                        games_extras::title.eq(&e.row.title),
                        games_extras::system.eq(&e.row.system),
                        games_extras::region.eq(&e.row.region),
                        games_extras::model.eq(&e.row.model),
                        games_extras::revision.eq(&e.row.revision),
                        games_extras::serial.eq(&e.row.serial),
                        games_extras::variant.eq(&e.row.variant),
                        games_extras::complete.eq(e.row.complete),
                        games_extras::modified.eq(e.row.modified),
                    ))
                    .on_conflict_do_nothing()
                    .execute(conn)
                    .await?;

                for (idx, game) in e.game.iter().enumerate() {
                    diesel::insert_into(games_extras_ref::table)
                        .values((
                            games_extras_ref::extra.eq(uid),
                            games_extras_ref::game.eq(DbUuid::from(*game)),
                            games_extras_ref::idx.eq(i64::try_from(idx).unwrap_or(i64::MAX)),
                        ))
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
