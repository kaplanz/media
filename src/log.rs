//! Logging.

use clap_verbosity_flag::{InfoLevel, Verbosity};
use tracing_subscriber::EnvFilter;
use tracing_subscriber::filter::LevelFilter;

/// Initializes the global tracing subscriber.
///
/// # Errors
///
/// Returns an error if a subscriber is already installed.
pub fn init(verbosity: &Verbosity<InfoLevel>) -> anyhow::Result<()> {
    let level = verbosity
        .tracing_level()
        .map_or(LevelFilter::OFF, LevelFilter::from);
    let filter = EnvFilter::builder()
        .with_default_directive(level.into())
        .from_env_lossy();
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .try_init()
        .map_err(|err| anyhow::anyhow!("failed to initialize logger: {err}"))
}
