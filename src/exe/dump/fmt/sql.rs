//! SQL dump format.

use std::fs::File;
use std::io::{BufWriter, Stdout, Write};

use anyhow::Context;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use either::Either;

use crate::db::{self, Pool};

#[derive(QueryableByName)]
struct Name {
    #[diesel(sql_type = diesel::sql_types::Text)]
    name: String,
}

#[derive(QueryableByName)]
struct Stmt {
    #[diesel(sql_type = diesel::sql_types::Text)]
    stmt: String,
}

pub async fn run(pool: &Pool, mut out: BufWriter<Either<File, Stdout>>) -> anyhow::Result<()> {
    let mut conn = db::get_conn(pool)
        .await
        .context("failed to get connection")?;

    writeln!(out, "BEGIN TRANSACTION;")?;

    let tables: Vec<Name> = diesel::sql_query(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
         ORDER BY rowid",
    )
    .load(&mut conn)
    .await
    .context("failed to list tables")?;

    for row in &tables {
        let table = &row.name;
        let columns: Vec<Name> =
            diesel::sql_query(format!("SELECT name FROM pragma_table_info('{table}')"))
                .load(&mut conn)
                .await
                .with_context(|| format!("failed to inspect {table}"))?;

        if columns.is_empty() {
            continue;
        }

        let col_list = columns
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        let quoted = columns
            .iter()
            .map(|c| format!("quote({})", c.name))
            .collect::<Vec<_>>()
            .join(" || ', ' || ");

        let rows: Vec<Stmt> = diesel::sql_query(format!(
            "SELECT 'INSERT INTO \"{table}\" ({col_list}) VALUES (' \
             || {quoted} || ');' AS stmt FROM \"{table}\""
        ))
        .load(&mut conn)
        .await
        .with_context(|| format!("failed to dump {table}"))?;

        for r in rows {
            writeln!(out, "{}", r.stmt)?;
        }
    }

    writeln!(out, "COMMIT;")?;
    Ok(())
}
