//! Dump command CLI.

use std::path::PathBuf;

use clap::ValueHint;

#[derive(Debug)]
#[derive(clap::Args)]
pub struct Cli {
    /// SQLite database file.
    #[arg(value_name = "DATABASE")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(env = "MEDIA_DB")]
    pub db: PathBuf,

    /// Output format.
    ///
    /// Selects the format used to serialize the media collection. If
    /// unspecified, defaults to JSON.
    #[arg(long = "format", short = 'f')]
    #[arg(visible_alias = "fmt")]
    #[arg(value_name = "FORMAT")]
    pub fmt: Option<Format>,

    /// Output file (default: stdout).
    ///
    /// An optional file for writing output. If unspecified or "-", the
    /// standard output stream is used.
    #[arg(long, short = 'o')]
    #[arg(value_name = "PATH")]
    #[arg(value_hint = ValueHint::FilePath)]
    pub output: Option<PathBuf>,
}

/// Serialization format.
#[derive(Copy, Clone, Debug, Default)]
#[derive(clap::ValueEnum)]
#[non_exhaustive]
pub enum Format {
    /// JSON array of records.
    #[default]
    Json,
    /// SQL INSERT statements.
    Sql,
}
