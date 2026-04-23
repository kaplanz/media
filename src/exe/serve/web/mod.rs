//! Web server.

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::{Extension, Router, middleware};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

mod auth;
pub mod route;

/// Builds and serves the application router on the given address.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot be bound or the server fails.
pub async fn serve(db: SqlitePool, addr: SocketAddr, token: Option<String>) -> anyhow::Result<()> {
    let app = Router::new()
        .nest("/books", route::books::router())
        .nest("/films", route::films::router())
        .nest("/games", route::games::router())
        .nest("/links", route::links::router())
        .nest("/shows", route::shows::router())
        .layer(middleware::from_fn(auth::guard))
        .layer(Extension(token.map(Arc::new)))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(db);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .with_context(|| format!("failed to bind {addr}"))?;

    tracing::info!("listening on {addr}");

    axum::serve(listener, app).await.context("server error")
}
