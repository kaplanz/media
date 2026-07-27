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

/// Partial request body.
///
/// Absent fields are left untouched; nullable fields set to `null` are
/// cleared.
#[derive(Clone, Debug, Default)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct Patch {
    /// URL.
    #[serde(deserialize_with = "crate::patch::present")]
    pub url: Option<String>,
    /// Title.
    #[serde(deserialize_with = "crate::patch::present")]
    pub title: Option<Option<String>>,
}

impl Patch {
    /// Returns `true` if no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.url.is_none() && self.title.is_none()
    }
}
