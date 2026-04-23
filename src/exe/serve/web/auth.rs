//! Authentication middleware.

use std::sync::Arc;

use axum::Extension;
use axum::extract::Request;
use axum::http::StatusCode;
use axum::http::header::AUTHORIZATION;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

/// Guards write operations behind a Bearer token.
///
/// `GET` requests are always permitted. All other methods require an
/// `Authorization: Bearer <token>` header matching the configured token.
/// If no token is configured, all requests are permitted.
pub async fn guard(
    Extension(token): Extension<Option<Arc<String>>>,
    req: Request,
    next: Next,
) -> Response {
    if req.method() == axum::http::Method::GET {
        return next.run(req).await;
    }
    let Some(ref expected) = token else {
        return next.run(req).await;
    };
    let bearer = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "));
    if bearer == Some(expected.as_str()) {
        next.run(req).await
    } else {
        StatusCode::UNAUTHORIZED.into_response()
    }
}
