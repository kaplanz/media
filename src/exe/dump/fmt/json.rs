//! JSON dump format.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Stdout, Write};

use anyhow::Context;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use either::Either;
use media::book::Book;
use media::film::Film;
use media::game::Game;
use media::game::copies::{Data, Row};
use media::game::extras::{Data as ExtrasData, Row as ExtrasRow};
use media::game::systems::{Data as SystemsData, Row as SystemsRow};
use media::link::Link;
use media::logs::Log;
use media::show::Show;
use media::{Item, Meta, Record};
use uuid::Uuid;

use crate::db::{self, Pool, Uuid as DbUuid};
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

/// Top-level dump payload.
#[derive(serde::Serialize, serde::Deserialize)]
pub struct Dump {
    pub media: Vec<Record<Item>>,
    #[serde(default, skip_serializing_if = "Games::is_empty")]
    pub games: Games,
}

/// Game subcollections.
#[derive(Default, serde::Serialize, serde::Deserialize)]
pub struct Games {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub systems: Vec<SystemsData>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub copies: Vec<Data>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub extras: Vec<ExtrasData>,
}

impl Games {
    fn is_empty(&self) -> bool {
        self.systems.is_empty() && self.copies.is_empty() && self.extras.is_empty()
    }
}

#[allow(clippy::too_many_lines)]
pub async fn run(pool: &Pool, mut out: BufWriter<Either<File, Stdout>>) -> anyhow::Result<()> {
    let mut conn = db::get_conn(pool)
        .await
        .context("failed to get connection")?;

    let rows: Vec<(DbUuid, String, i64, i64)> = m::table
        .select((m::id, m::kind, m::created, m::updated))
        .order_by(m::created.asc())
        .load(&mut conn)
        .await
        .context("failed to query media")?;

    let ids: Vec<DbUuid> = rows.iter().map(|r| r.0).collect();

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

    let mut items: HashMap<Uuid, Item> = HashMap::new();

    if !book.is_empty() {
        for b in books::table
            .filter(books::id.eq_any(&book))
            .select(books::all_columns)
            .load::<Book>(&mut conn)
            .await
            .context("failed to query books")?
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
            .context("failed to query films")?
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
            .context("failed to query games")?
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
            .context("failed to query links")?
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
            .context("failed to query shows")?
        {
            items.insert(s.id, Item::Show(s));
        }
    }

    let pairs: Vec<(DbUuid, String)> = tags::table
        .filter(tags::media.eq_any(&ids))
        .select((tags::media, tags::label))
        .order_by((tags::media, tags::label))
        .load(&mut conn)
        .await
        .context("failed to query tags")?;

    let mut tags: HashMap<Uuid, Vec<String>> = HashMap::new();
    for (uid, label) in pairs {
        tags.entry(uid.into()).or_default().push(label);
    }

    let pairs: Vec<(DbUuid, Log)> = logs::table
        .filter(logs::media.eq_any(&ids))
        .select((logs::media, (logs::id, logs::kind, logs::date)))
        .order_by((logs::media, logs::date))
        .load(&mut conn)
        .await
        .context("failed to query logs")?;

    let mut logs: HashMap<Uuid, Vec<Log>> = HashMap::new();
    for (uid, log) in pairs {
        logs.entry(uid.into()).or_default().push(log);
    }

    let media: Vec<Record<Item>> = rows
        .into_iter()
        .filter_map(|(uid, _, created, updated)| {
            let id: Uuid = uid.into();
            let item = items.remove(&id)?;
            let tags = tags.remove(&id).unwrap_or_default();
            let logs = logs.remove(&id).unwrap_or_default();
            Some(Record {
                item,
                meta: Meta { created, updated },
                logs,
                tags,
            })
        })
        .collect();

    let rows: Vec<SystemsRow> = games_systems::table
        .select(games_systems::all_columns)
        .order_by(games_systems::title.asc())
        .load(&mut conn)
        .await
        .context("failed to query games_systems")?;

    let pairs: Vec<(DbUuid, DbUuid)> = games_systems_ref::table
        .select((games_systems_ref::system, games_systems_ref::game))
        .order_by((games_systems_ref::system, games_systems_ref::idx))
        .load(&mut conn)
        .await
        .context("failed to query games_systems_ref")?;

    let mut refs: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (owner, game) in pairs {
        refs.entry(owner.into()).or_default().push(game.into());
    }

    let systems: Vec<SystemsData> = rows
        .into_iter()
        .map(|row| {
            let game = refs.remove(&row.id).unwrap_or_default();
            SystemsData { row, game }
        })
        .collect();

    let rows: Vec<Row> = games_copies::table
        .select(games_copies::all_columns)
        .order_by((games_copies::system.asc(), games_copies::title.asc()))
        .load(&mut conn)
        .await
        .context("failed to query games_copies")?;

    let pairs: Vec<(DbUuid, DbUuid)> = games_copies_ref::table
        .select((games_copies_ref::copy, games_copies_ref::game))
        .order_by((games_copies_ref::copy, games_copies_ref::idx))
        .load(&mut conn)
        .await
        .context("failed to query games_copies_ref")?;

    let mut works: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (copy, game) in pairs {
        works.entry(copy.into()).or_default().push(game.into());
    }

    let copies: Vec<Data> = rows
        .into_iter()
        .map(|row| {
            let game = works.remove(&row.id).unwrap_or_default();
            Data { row, game }
        })
        .collect();

    let rows: Vec<ExtrasRow> = games_extras::table
        .select(games_extras::all_columns)
        .order_by(games_extras::title.asc())
        .load(&mut conn)
        .await
        .context("failed to query games_extras")?;

    let pairs: Vec<(DbUuid, DbUuid)> = games_extras_ref::table
        .select((games_extras_ref::extra, games_extras_ref::game))
        .order_by((games_extras_ref::extra, games_extras_ref::idx))
        .load(&mut conn)
        .await
        .context("failed to query games_extras_ref")?;

    let mut refs: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for (owner, game) in pairs {
        refs.entry(owner.into()).or_default().push(game.into());
    }

    let extras: Vec<ExtrasData> = rows
        .into_iter()
        .map(|row| {
            let game = refs.remove(&row.id).unwrap_or_default();
            ExtrasData { row, game }
        })
        .collect();

    let dump = Dump {
        media,
        games: Games {
            systems,
            copies,
            extras,
        },
    };

    serde_json::to_writer_pretty(&mut out, &dump).context("failed to serialize")?;
    writeln!(out)?;
    Ok(())
}
