//! Watched film.

use uuid::Uuid;

/// Watched film.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Film {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// TMDB ID.
    pub tmdb: Option<i64>,
    /// Title.
    pub title: String,
    /// Release year.
    pub year: Option<i64>,
    /// Rating (1-5).
    pub rated: Option<i64>,
}

impl Film {
    pub const KIND: crate::Kind = crate::Kind::Film;
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// TMDB ID.
    pub tmdb: Option<i64>,
    /// Title.
    pub title: String,
    /// Release year.
    pub year: Option<i64>,
    /// Rating (1-5).
    pub rated: Option<i64>,
}
