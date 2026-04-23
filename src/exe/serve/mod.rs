//! Serve subcommand.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::str::FromStr;

use anyhow::Context;
use merge::Merge;

pub use self::cli::Cli;
use crate::db;

mod cli;
mod web;

const HOST: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
const PORT: u16 = 3000;

/// Runs the serve subcommand.
///
/// # Errors
///
/// Returns an error if configuration loading, database connection, or serving
/// fails.
pub fn main(mut args: Cli) -> crate::err::Result<()> {
    // NOTE: CLI args take priority over config file values.
    args.cfg.merge(crate::cfg::load(&args.config)?);

    let host = args
        .cfg
        .host
        .as_deref()
        .map(IpAddr::from_str)
        .transpose()
        .with_context(|| format!("invalid host: {}", args.cfg.host.as_deref().unwrap_or("")))?
        .unwrap_or(HOST);
    let port = args.cfg.port.unwrap_or(PORT);
    let addr = SocketAddr::new(host, port);

    // Multi-threaded runtime required for concurrent connections.
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?
        .block_on(async {
            let path = args.db.to_string_lossy();
            db::run_migrations(&path)
                .with_context(|| format!("failed to run migrations on {}", args.db.display()))?;
            let pool = db::build_pool(&path)
                .await
                .with_context(|| format!("failed to connect to {}", args.db.display()))?;
            if args.cfg.token.is_none() {
                tracing::warn!("no API key configured");
                tracing::warn!("running in read-only mode");
            }
            web::serve(pool, addr, args.cfg.token, args.cfg.prefix).await
        })
        .map_err(Into::into)
}
