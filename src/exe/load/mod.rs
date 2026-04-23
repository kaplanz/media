//! Load subcommand.

use std::fs::File;
use std::io::{BufReader, Stdin};
use std::path::Path;

use anyhow::Context;
use either::Either;

pub use self::cli::Cli;
use crate::db::{self, Pool};
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
    // Single-threaded runtime suffices for a one-shot operation.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?
        .block_on(async {
            let db_url = args.db.to_string_lossy();
            // NOTE: Migrations must run before loading data.
            db::run_migrations(&db_url)
                .with_context(|| format!("failed to run migrations on {}", args.db.display()))?;
            let pool: Pool = db::build_pool(&db_url)
                .await
                .with_context(|| format!("failed to open {}", args.db.display()))?;

            let reader: BufReader<Either<File, Stdin>> =
                BufReader::new(match args.input.as_deref() {
                    Some(path) if path != Path::new("-") => Either::Left(
                        File::open(path)
                            .with_context(|| format!("failed to open {}", path.display()))?,
                    ),
                    _ => Either::Right(std::io::stdin()),
                });

            let fmt = args.fmt.unwrap_or_else(|| {
                Format::infer(args.input.as_deref().filter(|p| *p != Path::new("-")))
            });
            match fmt {
                Format::Json => fmt::json::run(&pool, reader).await,
                Format::Sql => fmt::sql::run(&db_url, reader),
            }
        })
        .map_err(Into::into)
}
