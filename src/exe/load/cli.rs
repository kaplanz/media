//! Load command CLI.

use std::path::PathBuf;

use clap::ValueHint;

use crate::exe::dump::Format;

#[derive(Debug)]
#[derive(clap::Args)]
pub struct Cli {
    /// SQLite database file.
    #[arg(value_name = "DATABASE")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(env = "MEDIA_DB")]
    pub db: PathBuf,

    /// Input format.
    ///
    /// Selects the format used to deserialize the media collection. If
    /// unspecified, defaults to JSON.
    #[arg(long = "format", short = 'f')]
    #[arg(visible_alias = "fmt")]
    #[arg(value_name = "FORMAT")]
    pub fmt: Option<Format>,

    /// Input file (default: stdin).
    ///
    /// An optional file for reading input. If unspecified or "-", the
    /// standard input stream is used.
    #[arg(long, short = 'i')]
    #[arg(value_name = "PATH")]
    #[arg(value_hint = ValueHint::FilePath)]
    pub input: Option<PathBuf>,
}
