//! SQL dump format.

use std::fs::File;
use std::io::{BufWriter, Stdout, Write};

use anyhow::Context;
use either::Either;
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    mut out: BufWriter<Either<File, Stdout>>,
) -> anyhow::Result<()> {
    writeln!(out, "BEGIN TRANSACTION;")?;

    let tables: Vec<String> = sqlx::query_scalar(
        "SELECT name FROM sqlite_master \
         WHERE type = 'table' AND name NOT LIKE 'sqlite_%' \
         ORDER BY rowid",
    )
    .fetch_all(pool)
    .await
    .context("failed to list tables")?;

    for table in &tables {
        let columns: Vec<String> =
            sqlx::query_scalar(&format!("SELECT name FROM pragma_table_info('{table}')"))
                .fetch_all(pool)
                .await
                .with_context(|| format!("failed to inspect {table}"))?;

        if columns.is_empty() {
            continue;
        }

        let col_list = columns.join(", ");
        let quoted = columns
            .iter()
            .map(|c| format!("quote({c})"))
            .collect::<Vec<_>>()
            .join(" || ', ' || ");

        let sql = format!(
            "SELECT 'INSERT INTO \"{table}\" ({col_list}) VALUES (' \
             || {quoted} || ');' FROM \"{table}\""
        );

        let rows: Vec<String> = sqlx::query_scalar(&sql)
            .fetch_all(pool)
            .await
            .with_context(|| format!("failed to dump {table}"))?;

        for row in rows {
            writeln!(out, "{row}")?;
        }
    }

    writeln!(out, "COMMIT;")?;
    Ok(())
}
