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

    /// Server bind address.
    #[arg(long, env = "MEDIA_HOST")]
    pub host: Option<String>,

    /// Server bind port.
    #[arg(long, env = "MEDIA_PORT")]
    pub port: Option<u16>,

    /// Bearer token required for write operations.
    #[arg(long, env = "MEDIA_TOKEN")]
    pub token: Option<String>,
}
