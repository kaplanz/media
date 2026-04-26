//! Load subcommand.

use std::fs::File;
use std::io::{BufReader, Stdin};
use std::path::Path;

use anyhow::Context;
use either::Either;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

pub use self::cli::Cli;
use crate::exe::dump::Format;

mod cli;
mod fmt;

/// Runs the load subcommand.
///
/// # Errors
///
/// Returns an error if the database cannot be opened or the load fails.
#[expect(clippy::needless_pass_by_value)]
pub fn main(args: Cli) -> crate::err::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?
        .block_on(async {
            let init = !args.db.exists();
            let opts = SqliteConnectOptions::new()
                .filename(&args.db)
                .create_if_missing(true)
                .foreign_keys(true)
                .pragma("journal_mode", "WAL");
            let pool = SqlitePool::connect_with(opts)
                .await
                .with_context(|| format!("failed to open {}", args.db.display()))?;
            if init {
                sqlx::raw_sql(include_str!("../../../sql/main.sql"))
                    .execute(&pool)
                    .await
                    .context("failed to initialize database")?;
            }

            let reader: BufReader<Either<File, Stdin>> =
                BufReader::new(match args.input.as_deref() {
                    Some(path) if path != Path::new("-") => Either::Left(
                        File::open(path)
                            .with_context(|| format!("failed to open {}", path.display()))?,
                    ),
                    _ => Either::Right(std::io::stdin()),
                });

            match args.fmt.unwrap_or_default() {
                Format::Json => fmt::json::run(&pool, reader).await,
                Format::Sql => fmt::sql::run(&pool, reader).await,
            }
        })
        .map_err(Into::into)
}
