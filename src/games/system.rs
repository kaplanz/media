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
    /// Model name.
    #[serde(deserialize_with = "crate::patch::present")]
    pub model: Option<Option<String>>,
    /// Hardware revision.
    #[serde(deserialize_with = "crate::patch::present")]
    pub revision: Option<Option<String>>,
    /// Serial number.
    #[serde(deserialize_with = "crate::patch::present")]
    pub serial: Option<Option<String>>,
    /// Variation.
    #[serde(deserialize_with = "crate::patch::present")]
    pub variation: Option<Option<String>>,
}

impl Patch {
    /// Returns `true` if no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.system.is_none()
            && self.model.is_none()
            && self.revision.is_none()
            && self.serial.is_none()
            && self.variation.is_none()
    }
}
