//! Serve command CLI.

use std::path::PathBuf;

use clap::ValueHint;

#[derive(Debug)]
#[derive(clap::Args)]
pub struct Cli {
    /// Path to configuration file.
    #[arg(long = "config", env = "MEDIA_CFG")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(default_value_os_t = crate::cfg::path())]
    pub config: PathBuf,

    /// SQLite database file.
    #[arg(value_name = "DATABASE")]
    #[arg(value_hint = ValueHint::FilePath)]
    #[arg(env = "MEDIA_DB")]
    pub db: PathBuf,

    /// Server configuration.
    #[command(flatten)]
    pub cfg: crate::cfg::Config,
}
