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
