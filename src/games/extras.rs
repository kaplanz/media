//! Game accessory types.

use uuid::Uuid;

/// Game accessory.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Extras {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// Title.
    pub title: String,
    /// Platform.
    pub system: Option<String>,
    /// Region code.
    pub region: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Variant.
    pub variant: Option<String>,
    /// Complete-in-box status.
    pub complete: bool,
    /// Hardware modification status.
    pub modified: bool,
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
    /// Region code.
    pub region: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
    /// Variant.
    pub variant: Option<String>,
    /// Complete-in-box status.
    pub complete: Option<bool>,
    /// Hardware modification status.
    pub modified: Option<bool>,
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
    /// Region code.
    #[serde(deserialize_with = "crate::patch::present")]
    pub region: Option<Option<String>>,
    /// Model name.
    #[serde(deserialize_with = "crate::patch::present")]
    pub model: Option<Option<String>>,
    /// Hardware revision.
    #[serde(deserialize_with = "crate::patch::present")]
    pub revision: Option<Option<String>>,
    /// Serial number.
    #[serde(deserialize_with = "crate::patch::present")]
    pub serial: Option<Option<String>>,
    /// Variant.
    #[serde(deserialize_with = "crate::patch::present")]
    pub variant: Option<Option<String>>,
    /// Complete-in-box status.
    #[serde(deserialize_with = "crate::patch::present")]
    pub complete: Option<bool>,
    /// Hardware modification status.
    #[serde(deserialize_with = "crate::patch::present")]
    pub modified: Option<bool>,
}

impl Patch {
    /// Returns `true` if no fields are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.title.is_none()
            && self.system.is_none()
            && self.region.is_none()
            && self.model.is_none()
            && self.revision.is_none()
            && self.serial.is_none()
            && self.variant.is_none()
            && self.complete.is_none()
            && self.modified.is_none()
    }
}
