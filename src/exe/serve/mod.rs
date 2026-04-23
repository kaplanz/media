//! Serve subcommand.

use std::net::{IpAddr, Ipv6Addr, SocketAddr};
use std::str::FromStr;

use anyhow::Context;
use merge::Merge;
use sqlx::SqlitePool;
use sqlx::sqlite::SqliteConnectOptions;

pub use self::cli::Cli;

mod cli;
mod web;

const HOST: IpAddr = IpAddr::V6(Ipv6Addr::LOCALHOST);
const PORT: u16 = 3000;

/// Runs the serve subcommand.
///
/// # Errors
///
/// Returns an error if configuration loading, database connection, or serving fails.
pub fn main(args: Cli) -> crate::err::Result<()> {
    let mut cfg = crate::cfg::Config {
        host: args.host,
        port: args.port,
        token: args.token,
    };
    cfg.merge(crate::cfg::load(&args.config)?);

    let db = args.db;
    let host = cfg
        .host
        .as_deref()
        .map(IpAddr::from_str)
        .transpose()
        .with_context(|| format!("invalid host: {}", cfg.host.as_deref().unwrap_or("")))?
        .unwrap_or(HOST);
    let port = cfg.port.unwrap_or(PORT);

    let addr = SocketAddr::new(host, port);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .context("failed to build runtime")?
        .block_on(async {
            let init = !db.exists();
            let opts = SqliteConnectOptions::new()
                .filename(&db)
                .create_if_missing(true)
                .foreign_keys(true)
                .pragma("journal_mode", "WAL");
            let pool = SqlitePool::connect_with(opts)
                .await
                .with_context(|| format!("failed to connect to {}", db.display()))?;
            if init {
                sqlx::raw_sql(include_str!("../../../sql/main.sql"))
                    .execute(&pool)
                    .await
                    .context("failed to initialize database")?;
            }
            web::serve(pool, addr, cfg.token).await
        })
        .map_err(Into::into)
}
