//! Dump subcommand.

use std::fs::File;
use std::io::{BufWriter, Stdout};
use std::path::Path;

use anyhow::Context;
use either::Either;

pub use self::cli::{Cli, Format};
pub use self::fmt::json::Dump;
use crate::db::{self, Pool};

mod cli;
mod fmt;

/// Runs the dump subcommand.
///
/// # Errors
///
/// Returns an error if the database cannot be opened or the dump fails.
#[expect(clippy::needless_pass_by_value)]
pub fn main(args: Cli) -> crate::err::Result<()> {
    // Single-threaded runtime suffices for a one-shot operation.
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?
        .block_on(async {
            let db_url = args.db.to_string_lossy();
            let pool: Pool = db::build_pool(&db_url)
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

            let fmt = args.fmt.unwrap_or_else(|| {
                Format::infer(args.output.as_deref().filter(|p| *p != Path::new("-")))
            });
            match fmt {
                Format::Json => fmt::json::run(&pool, writer).await,
                Format::Sql => fmt::sql::run(&pool, writer).await,
            }
        })
        .map_err(Into::into)
}
