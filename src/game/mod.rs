//! Video game.

pub mod extras;
pub mod owned;
pub mod system;

use uuid::Uuid;

/// Video game.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Game {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// Title.
    pub title: String,
    /// Platform.
    pub system: Option<String>,
    /// Rating (1-5).
    pub rating: Option<i64>,
}

impl Game {
    pub const KIND: crate::Kind = crate::Kind::Game;
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// Title.
    pub title: String,
    /// Platform.
    pub system: Option<String>,
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
    /// Title.
    #[serde(deserialize_with = "crate::patch::present")]
    pub title: Option<String>,
    /// Platform.
    #[serde(deserialize_with = "crate::patch::present")]
    pub system: Option<Option<String>>,
    /// Rating (1-5).
    #[serde(deserialize_with = "crate::patch::present")]
    pub rating: Option<Option<i64>>,
}

impl Patch {
    /// Returns `true` if no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_none() && self.system.is_none() && self.rating.is_none()
    }
}
