//! Web bookmark.

use uuid::Uuid;

/// Web bookmark.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Link {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// URL.
    pub url: String,
    /// Title.
    pub title: Option<String>,
}

impl Link {
    pub const KIND: crate::Kind = crate::Kind::Link;
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// URL.
    pub url: String,
    /// Title.
    pub title: Option<String>,
}
