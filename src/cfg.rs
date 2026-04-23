//! Application configuration.

use std::fs;
use std::io::ErrorKind::NotFound;
use std::path::{Path, PathBuf};

use merge::Merge;
use serde::Deserialize;

/// Application configuration data.
#[derive(Debug, Default, Deserialize, Merge)]
#[derive(clap::Args)]
pub struct Config {
    /// Server bind address.
    #[arg(long, env = "HOST")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub host: Option<String>,
    /// Server bind port.
    #[arg(long, env = "PORT")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub port: Option<u16>,
    /// Bearer token required for write operations.
    #[arg(long, env = "TOKEN")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub token: Option<String>,
    /// URL prefix when served behind a reverse proxy.
    #[arg(long, env = "PREFIX")]
    #[merge(strategy = merge::option::overwrite_none)]
    pub prefix: Option<String>,
}

/// Returns the path to the application's configuration file.
#[must_use]
pub fn path() -> PathBuf {
    crate::dir::config().join("config.toml")
}

/// Loads configuration data from a file.
///
/// # Errors
///
/// This function will return an error if the configuration could not be
/// loaded.
pub fn load(path: &Path) -> Result<Config> {
    match fs::read_to_string(path) {
        // NOTE: Missing file is not an error; defaults fill unset fields.
        Err(err) if err.kind() == NotFound => Ok(String::default()),
        Err(err) => Err(err.into()),
        Ok(body) => Ok(body),
    }
    .and_then(|body| toml::from_str(&body).map_err(Into::into))
}

/// A convenient type alias for [`Result`](std::result::Result).
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// An error caused by [loading](load) the configuration.
#[derive(Debug)]
#[derive(thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Reading error.
    #[error("reading configuration failed")]
    Read(#[from] std::io::Error),
    /// Parsing error.
    #[error("parsing configuration failed")]
    Parse(#[from] toml::de::Error),
}
