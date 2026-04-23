//! Web bookmark.

use uuid::Uuid;

/// Web bookmark.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Link {
    /// Unique identifier.
    pub id: Uuid,
    /// URL.
    pub url: String,
    /// Title.
    pub title: Option<String>,
}

/// Request body for creating or updating a hyperlink.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// URL.
    pub url: String,
    /// Title.
    pub title: Option<String>,
}
