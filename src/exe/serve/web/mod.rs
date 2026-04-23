//! Web server.

#![allow(clippy::needless_for_each)]

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::routing::get;
use axum::{Extension, Json, middleware};
use sqlx::SqlitePool;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter as Router;
use utoipa_scalar::{Scalar, Servable as _};

mod auth;
pub mod route;

/// REST API for managing a personal media collection.
///
/// Supports books, films, games, links, and television shows. Each kind
/// has its own set of endpoints for listing, fetching, creating, updating,
/// and deleting records. All list endpoints support filtering, sorting, and
/// pagination via query parameters.
#[derive(OpenApi)]
#[openapi(tags(
    (name = "books", description = "Reading items."),
    (name = "films", description = "Watched films."),
    (name = "games", description = "Video games."),
    (name = "links", description = "Web bookmarks."),
    (name = "shows", description = "Television shows."),
))]
struct Doc;

/// Builds and serves the application router on the given address.
///
/// # Errors
///
/// Returns an error if the TCP listener cannot be bound or the server fails.
pub async fn serve(db: SqlitePool, addr: SocketAddr, token: Option<String>) -> anyhow::Result<()> {
    let (router, api) = Router::with_openapi(Doc::openapi())
        .nest("/books", route::books::router())
        .nest("/films", route::films::router())
        .nest("/games", route::games::router())
        .nest("/links", route::links::router())
        .nest("/shows", route::shows::router())
        .split_for_parts();

    let api = Arc::new(api);

    let app = router
        .route(
            "/openapi.json",
            get({
                let api = Arc::clone(&api);
                move || async move { Json((*api).clone()) }
            }),
        )
        .merge(Scalar::with_url("/docs", (*api).clone()))
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
