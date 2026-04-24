//! JSON load format.

use std::fs::File;
use std::io::{BufReader, Stdin};

use anyhow::Context;
use either::Either;
use media::kind::{Kind, Record};
use sqlx::SqlitePool;

#[expect(clippy::too_many_lines)]
pub async fn run(pool: &SqlitePool, reader: BufReader<Either<File, Stdin>>) -> anyhow::Result<()> {
    let records: Vec<Record> = serde_json::from_reader(reader).context("failed to parse JSON")?;

    let mut tx = pool.begin().await.context("failed to begin transaction")?;

    for record in &records {
        let (kind, id) = match &record.item {
            Kind::Book(b) => ("book", b.id),
            Kind::Film(f) => ("film", f.id),
            Kind::Game(g) => ("game", g.id),
            Kind::Link(l) => ("link", l.id),
            Kind::Show(s) => ("show", s.id),
        };

        sqlx::query(
            "INSERT OR IGNORE INTO media (id, kind, created, updated) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(id)
        .bind(kind)
        .bind(record.meta.created)
        .bind(record.meta.updated)
        .execute(&mut *tx)
        .await
        .context("failed to insert media")?;

        match &record.item {
            Kind::Book(b) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO books \
                     (id, isbn, hcid, title, cover, about, color) \
                     VALUES (?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(b.id)
                .bind(&b.isbn)
                .bind(b.hcid)
                .bind(&b.title)
                .bind(&b.cover)
                .bind(&b.about)
                .bind(&b.color)
                .execute(&mut *tx)
                .await
                .context("failed to insert book")?;
            }
            Kind::Film(f) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO films \
                     (id, tmdb, title, year, rated) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(f.id)
                .bind(f.tmdb)
                .bind(&f.title)
                .bind(f.year)
                .bind(f.rated)
                .execute(&mut *tx)
                .await
                .context("failed to insert film")?;
            }
            Kind::Game(g) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO games \
                     (id, tgdb, title, system, owned, rated) \
                     VALUES (?, ?, ?, ?, ?, ?)",
                )
                .bind(g.id)
                .bind(g.tgdb)
                .bind(&g.title)
                .bind(&g.system)
                .bind(g.owned)
                .bind(g.rated)
                .execute(&mut *tx)
                .await
                .context("failed to insert game")?;
            }
            Kind::Link(l) => {
                sqlx::query("INSERT OR IGNORE INTO links (id, url, title) VALUES (?, ?, ?)")
                    .bind(l.id)
                    .bind(&l.url)
                    .bind(&l.title)
                    .execute(&mut *tx)
                    .await
                    .context("failed to insert link")?;
            }
            Kind::Show(s) => {
                sqlx::query(
                    "INSERT OR IGNORE INTO shows \
                     (id, tmdb, title, year, rated) \
                     VALUES (?, ?, ?, ?, ?)",
                )
                .bind(s.id)
                .bind(s.tmdb)
                .bind(&s.title)
                .bind(s.year)
                .bind(s.rated)
                .execute(&mut *tx)
                .await
                .context("failed to insert show")?;
            }
        }

        for label in &record.tags {
            sqlx::query("INSERT OR IGNORE INTO tags (media, label) VALUES (?, ?)")
                .bind(id)
                .bind(label)
                .execute(&mut *tx)
                .await
                .context("failed to insert tag")?;
        }
    }

    tx.commit().await.context("failed to commit")?;
    Ok(())
}
