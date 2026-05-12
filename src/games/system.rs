//! Game console types.

use uuid::Uuid;

/// Owned game console.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct System {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// Title.
    pub title: String,
    /// Platform.
    pub system: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Variation.
    pub variation: Option<String>,
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
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Variation.
    pub variation: Option<String>,
}
