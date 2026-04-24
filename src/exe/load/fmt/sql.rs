//! SQL load format.

use std::fs::File;
use std::io::{BufReader, Read, Stdin};

use anyhow::Context;
use either::Either;
use sqlx::SqlitePool;

pub async fn run(
    pool: &SqlitePool,
    mut reader: BufReader<Either<File, Stdin>>,
) -> anyhow::Result<()> {
    let mut sql = String::new();
    reader
        .read_to_string(&mut sql)
        .context("failed to read input")?;
    sqlx::raw_sql(&sql)
        .execute(pool)
        .await
        .context("failed to execute SQL")?;
    Ok(())
}
