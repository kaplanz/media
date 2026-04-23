//! Application configuration.

use std::fs;
use std::io::ErrorKind::NotFound;
use std::path::{Path, PathBuf};

use merge::Merge;
use serde::Deserialize;

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

/// Application configuration data.
#[derive(Debug, Default, Deserialize, Merge)]
pub struct Config {
    #[merge(strategy = merge::option::overwrite_none)]
    pub host: Option<String>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub port: Option<u16>,
    #[merge(strategy = merge::option::overwrite_none)]
    pub token: Option<String>,
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
        // If the configuration file does not exist, return an empty string,
        // resulting in all fields being populated with defaults.
        Err(err) if err.kind() == NotFound => Ok(String::default()),
        // For other errors, return them directly.
        Err(err) => Err(err.into()),
        // On success, the body of the file can be parsed.
        Ok(body) => Ok(body),
    }
    .and_then(|body| {
        // If a configuration file was read, parse it.
        toml::from_str(&body)
            // Parsing errors should be mapped into a separate variant.
            .map_err(Into::into)
    })
}
