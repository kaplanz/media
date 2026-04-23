//! SQL load format.

use std::fs::File;
use std::io::{BufReader, Read, Stdin};

use anyhow::Context;
use diesel::Connection;
use diesel::connection::SimpleConnection as _;
use either::Either;

pub fn run(url: &str, mut reader: BufReader<Either<File, Stdin>>) -> anyhow::Result<()> {
    let mut sql = String::new();
    reader
        .read_to_string(&mut sql)
        .context("failed to read input")?;
    let mut conn = diesel::SqliteConnection::establish(url).context("failed to open database")?;
    conn.batch_execute(&sql).context("failed to execute SQL")?;
    Ok(())
}
