//! Web server.

#![allow(clippy::needless_for_each)]

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use axum::routing::get;
use axum::{Extension, Json, middleware};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa::openapi::security::{Http, HttpAuthScheme, SecurityScheme};
use utoipa_axum::router::OpenApiRouter as Router;

use crate::db::Pool;

mod auth;
mod error;
pub mod route;

/// REST API for managing a personal media collection.
///
/// Supports books, films, games, links, and television shows. Each kind has its
/// own set of endpoints for listing, fetching, creating, updating, and deleting
/// records. All list endpoints support filtering, sorting, and pagination via
/// query parameters.
#[derive(OpenApi)]
#[openapi(tags(
    (name = "media", description = "Any media item."),
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
pub async fn serve(
    db: Pool,
    addr: SocketAddr,
    token: Option<String>,
    prefix: Option<String>,
) -> anyhow::Result<()> {
    let (router, mut api) = Router::with_openapi(Doc::openapi())
        .merge(route::router())
        .merge(route::tags::router())
        .nest("/books", route::books::router())
        .nest("/films", route::films::router())
        .nest("/games", route::games::router())
        .nest("/links", route::links::router())
        .nest("/shows", route::shows::router())
        .split_for_parts();

    api.components
        .get_or_insert_with(Default::default)
        .add_security_scheme(
            "BearerAuth",
            SecurityScheme::Http(Http::new(HttpAuthScheme::Bearer)),
        );

    for path in api.paths.paths.values_mut() {
        for op in [
            path.get.as_mut(),
            path.post.as_mut(),
            path.put.as_mut(),
            path.delete.as_mut(),
            path.patch.as_mut(),
        ]
        .into_iter()
        .flatten()
        {
            if op.description.is_none() {
                op.description = op.summary.take();
            }
        }
    }

    if let Some(ref prefix) = prefix {
        api.servers = Some(vec![utoipa::openapi::Server::new(prefix)]);
    }

    let api = Arc::new(api);

    let app = router
        .route(
            "/openapi.json",
            get({
                let api = Arc::clone(&api);
                move || async move { Json((*api).clone()) }
            }),
        )
        .layer(middleware::from_fn(auth::guard))
        .layer(middleware::from_fn(error::handle))
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
