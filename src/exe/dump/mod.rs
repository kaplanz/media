//! Dump subcommand.

use std::fs::File;
use std::io::{BufWriter, Stdout};
use std::path::Path;

use anyhow::Context;
use either::Either;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

pub use self::cli::{Cli, Format};

mod cli;
mod fmt;

/// Runs the dump subcommand.
///
/// # Errors
///
/// Returns an error if the database cannot be opened or the dump fails.
#[expect(clippy::needless_pass_by_value)]
pub fn main(args: Cli) -> crate::err::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?
        .block_on(async {
            let opts = SqliteConnectOptions::new()
                .filename(&args.db)
                .read_only(true);
            let pool = SqlitePool::connect_with(opts)
                .await
                .with_context(|| format!("failed to open {}", args.db.display()))?;

            let writer: BufWriter<Either<File, Stdout>> =
                BufWriter::new(match args.output.as_deref() {
                    Some(path) if path != Path::new("-") => Either::Left(
                        File::create(path)
                            .with_context(|| format!("failed to create {}", path.display()))?,
                    ),
                    _ => Either::Right(std::io::stdout()),
                });

            match args.fmt.unwrap_or_default() {
                Format::Json => fmt::json::run(&pool, writer).await,
                Format::Sql => fmt::sql::run(&pool, writer).await,
            }
        })
        .map_err(Into::into)
}
