//! Error response middleware.

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::axum::extract::Body;

/// Converts plain-text error responses to JSON.
pub async fn handle(req: Request, next: Next) -> Response {
    let response = next.run(req).await;
    let status = response.status();
    if !status.is_client_error() && !status.is_server_error() {
        return response;
    }
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default();
    if content_type.contains("application/json") {
        return response;
    }
    let bytes = axum::body::to_bytes(response.into_body(), 16 * 1024)
        .await
        .unwrap_or_default();
    let error = if bytes.is_empty() {
        status.canonical_reason().unwrap_or("error").to_owned()
    } else {
        String::from_utf8_lossy(&bytes).trim().to_owned()
    };
    Body::reject(status, error, vec![]).into_response()
}
