//! Television show.

use uuid::Uuid;

/// Television show.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Show {
    /// Unique identifier.
    pub id: Uuid,
    /// TMDB ID.
    pub tmdb: Option<i64>,
    /// Title.
    pub title: String,
    /// First air year.
    pub year: Option<i64>,
    /// Rating (1-5).
    pub rated: Option<i64>,
}

/// Request body for creating or updating a television show.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// TMDB ID.
    pub tmdb: Option<i64>,
    /// Title.
    pub title: String,
    /// First air year.
    pub year: Option<i64>,
    /// Rating (1-5).
    pub rated: Option<i64>,
}
