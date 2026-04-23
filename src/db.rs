//! Database utilities.

use std::ops::DerefMut;

use diesel::Connection;
use diesel_async::pooled_connection::bb8::Pool as Bb8Pool;
use diesel_async::pooled_connection::{AsyncDieselConnectionManager, ManagerConfig};
use diesel_async::sync_connection_wrapper::SyncConnectionWrapper;
use diesel_async::{AsyncConnection as _, RunQueryDsl};
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
pub use media::Uuid;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!("sql/migrations");

pub type Conn = SyncConnectionWrapper<diesel::SqliteConnection>;

/// `bb8::Pool<AsyncDieselConnectionManager<Conn>>`
pub type Pool = Bb8Pool<Conn>;

/// Run pending migrations against the database at the given URL.
///
/// # Errors
///
/// Returns an error if the database cannot be opened or a migration fails.
pub fn run_migrations(url: &str) -> anyhow::Result<()> {
    diesel::SqliteConnection::establish(url)?
        .run_pending_migrations(MIGRATIONS)
        .map(|_| ())
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Build a connection pool for the given database URL.
///
/// # Errors
///
/// Returns an error if the pool manager cannot be initialized.
pub async fn build_pool(url: &str) -> anyhow::Result<Pool> {
    let mut config = ManagerConfig::default();
    config.custom_setup = Box::new(|url: &str| {
        Box::pin(async move {
            let mut conn = Conn::establish(url).await?;
            let _: usize = diesel::sql_query("PRAGMA foreign_keys = ON")
                .execute(&mut conn)
                .await
                .map_err(diesel::result::ConnectionError::CouldntSetupConfiguration)?;
            let _: usize = diesel::sql_query("PRAGMA journal_mode = WAL")
                .execute(&mut conn)
                .await
                .map_err(diesel::result::ConnectionError::CouldntSetupConfiguration)?;
            Ok(conn)
        })
    });
    let manager = AsyncDieselConnectionManager::<Conn>::new_with_config(url, config);
    Pool::builder()
        .build(manager)
        .await
        .map_err(|e| anyhow::anyhow!("{e}"))
}

/// Acquire a connection from the pool.
///
/// # Errors
///
/// Returns an error if a connection cannot be obtained from the pool.
pub async fn get_conn(pool: &Pool) -> anyhow::Result<impl DerefMut<Target = Conn> + '_> {
    pool.get().await.map_err(|e| anyhow::anyhow!("{e}"))
}

/// Current Unix timestamp in seconds.
///
/// # Panics
///
/// Panics if the system clock is set before the Unix epoch.
#[must_use]
#[allow(clippy::cast_possible_wrap)]
pub fn timestamp() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_secs() as i64
}
