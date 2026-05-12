//! Owned game release types.

use uuid::Uuid;

/// Owned physical game release.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Owned {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// Game reference.
    #[diesel(deserialize_as = crate::Uuid)]
    pub game: Uuid,
    /// Platform.
    pub system: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Complete-in-box status.
    pub cib: i64,
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// Game reference.
    pub game: Uuid,
    /// Platform.
    pub system: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Complete-in-box status.
    pub cib: Option<i64>,
}
