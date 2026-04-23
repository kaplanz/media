//! Reading item.

use uuid::Uuid;

/// Reading item.
#[derive(Clone, Debug)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Book {
    /// Unique identifier.
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

/// Request body for creating or updating a reading item.
#[derive(Clone, Debug)]
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
