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
    pub rating: Option<i64>,
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
    pub rating: Option<i64>,
}

/// Partial request body.
///
/// Absent fields are left untouched; nullable fields set to `null` are
/// cleared.
#[derive(Clone, Debug, Default)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Patch {
    /// TMDB ID.
    #[serde(deserialize_with = "crate::patch::present")]
    pub tmdb: Option<Option<i64>>,
    /// Title.
    #[serde(deserialize_with = "crate::patch::present")]
    pub title: Option<String>,
    /// Release year.
    #[serde(deserialize_with = "crate::patch::present")]
    pub year: Option<Option<i64>>,
    /// Rating (1-5).
    #[serde(deserialize_with = "crate::patch::present")]
    pub rating: Option<Option<i64>>,
}

impl Patch {
    /// Returns `true` if no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.tmdb.is_none() && self.title.is_none() && self.year.is_none() && self.rating.is_none()
    }
}
