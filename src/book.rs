//! Reading item.

use uuid::Uuid;

/// Reading item.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Book {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// ISBN-13.
    pub isbn: Option<String>,
    /// Hardcover ID.
    pub hcid: Option<i64>,
    /// Title.
    pub title: String,
    /// Cover image URL.
    pub cover: Option<String>,
    /// Description.
    pub about: Option<String>,
    /// Accent color.
    pub color: Option<String>,
}

impl Book {
    pub const KIND: crate::Kind = crate::Kind::Book;
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// ISBN-13.
    pub isbn: Option<String>,
    /// Hardcover ID.
    pub hcid: Option<i64>,
    /// Title.
    pub title: String,
    /// Cover image URL.
    pub cover: Option<String>,
    /// Description.
    pub about: Option<String>,
    /// Accent color.
    pub color: Option<String>,
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
    /// ISBN-13.
    #[serde(deserialize_with = "crate::patch::present")]
    pub isbn: Option<Option<String>>,
    /// Hardcover ID.
    #[serde(deserialize_with = "crate::patch::present")]
    pub hcid: Option<Option<i64>>,
    /// Title.
    #[serde(deserialize_with = "crate::patch::present")]
    pub title: Option<String>,
    /// Cover image URL.
    #[serde(deserialize_with = "crate::patch::present")]
    pub cover: Option<Option<String>>,
    /// Description.
    #[serde(deserialize_with = "crate::patch::present")]
    pub about: Option<Option<String>>,
    /// Accent color.
    #[serde(deserialize_with = "crate::patch::present")]
    pub color: Option<Option<String>>,
}

impl Patch {
    /// Returns `true` if no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.isbn.is_none()
            && self.hcid.is_none()
            && self.title.is_none()
            && self.cover.is_none()
            && self.about.is_none()
            && self.color.is_none()
    }
}
