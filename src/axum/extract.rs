//! Custom extractors with structured error responses.

use std::error::Error as StdError;

use axum::extract::FromRequestParts;
use axum::extract::rejection::{JsonRejection, PathRejection};
use axum::http::StatusCode;
use axum::http::request::Parts;
use axum::response::{IntoResponse, Response};
use serde::de::DeserializeOwned;

#[derive(serde::Serialize)]
pub struct Body {
    pub error: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub cause: Vec<String>,
}

impl Body {
    pub fn reject(status: StatusCode, error: impl Into<String>, cause: Vec<String>) -> Response {
        (
            status,
            axum::Json(Self {
                error: error.into(),
                cause,
            }),
        )
            .into_response()
    }
}

fn cause_chain(err: &dyn StdError) -> Vec<String> {
    let mut causes = Vec::new();
    let mut src = err.source();
    while let Some(e) = src {
        causes.push(e.to_string());
        src = e.source();
    }
    causes
}

fn full_cause_chain(err: &dyn StdError) -> Vec<String> {
    let mut causes = vec![err.to_string()];
    causes.extend(cause_chain(err));
    causes
}

/// Handler error response.
pub enum Error {
    /// 404 Not Found.
    NotFound,
    /// 500 Internal Server Error with database cause chain.
    Internal(sqlx::Error),
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Self::NotFound => Body::reject(
                StatusCode::NOT_FOUND,
                StatusCode::NOT_FOUND
                    .canonical_reason()
                    .unwrap_or("Not Found"),
                vec![],
            ),
            Self::Internal(e) => Body::reject(
                StatusCode::INTERNAL_SERVER_ERROR,
                e.to_string(),
                cause_chain(&e),
            ),
        }
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Internal(e)
    }
}

/// JSON body extractor with structured rejection responses.
pub struct Json<T>(pub T);

impl<T, S> axum::extract::FromRequest<S> for Json<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request(req: axum::extract::Request, state: &S) -> Result<Self, Self::Rejection> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(val)) => Ok(Self(val)),
            Err(rejection) => Err(match rejection {
                JsonRejection::JsonDataError(e) => Body::reject(
                    StatusCode::UNPROCESSABLE_ENTITY,
                    "invalid request body",
                    full_cause_chain(&e),
                ),
                JsonRejection::JsonSyntaxError(e) => Body::reject(
                    StatusCode::BAD_REQUEST,
                    "malformed JSON",
                    full_cause_chain(&e),
                ),
                JsonRejection::MissingJsonContentType(_) => Body::reject(
                    StatusCode::UNSUPPORTED_MEDIA_TYPE,
                    "expected Content-Type: application/json",
                    vec![],
                ),
                _ => Body::reject(StatusCode::INTERNAL_SERVER_ERROR, "internal error", vec![]),
            }),
        }
    }
}

impl<T: serde::Serialize> IntoResponse for Json<T> {
    fn into_response(self) -> Response {
        axum::Json(self.0).into_response()
    }
}

/// Path extractor with structured rejection responses.
pub struct Path<T>(pub T);

impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match axum::extract::Path::<T>::from_request_parts(parts, state).await {
            Ok(axum::extract::Path(val)) => Ok(Self(val)),
            Err(rejection) => Err(match rejection {
                PathRejection::FailedToDeserializePathParams(e) => Body::reject(
                    StatusCode::BAD_REQUEST,
                    "invalid path parameter",
                    full_cause_chain(&e),
                ),
                PathRejection::MissingPathParams(_) => Body::reject(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "missing path parameters",
                    vec![],
                ),
                _ => Body::reject(StatusCode::INTERNAL_SERVER_ERROR, "internal error", vec![]),
            }),
        }
    }
}
