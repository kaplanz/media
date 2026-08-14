//! Game console types.

use uuid::Uuid;

use super::Game;

/// Owned game console.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Systems {
    /// Unique identifier.
    pub id: Uuid,
    /// Title.
    pub title: String,
    /// Included games.
    pub game: Vec<Game>,
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

/// Stored game console.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Row {
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

impl Row {
    /// Resolves the game references into a full game console.
    #[must_use]
    pub fn resolve(self, game: Vec<Game>) -> Systems {
        Systems {
            id: self.id,
            title: self.title,
            game,
            system: self.system,
            region: self.region,
            model: self.model,
            revision: self.revision,
            serial: self.serial,
            variant: self.variant,
            complete: self.complete,
            modified: self.modified,
        }
    }
}

/// Stored game console with its game references.
#[derive(Clone, Debug)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Data {
    /// Systems data.
    #[serde(flatten)]
    pub row: Row,
    /// Game references.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub game: Vec<Uuid>,
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// Title.
    pub title: String,
    /// Included games.
    #[serde(default)]
    pub game: Vec<Uuid>,
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
    /// Included games.
    #[serde(deserialize_with = "crate::patch::present")]
    pub game: Option<Vec<Uuid>>,
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
        self.game.is_none() && !self.has_columns()
    }

    /// Returns `true` if any column field is present.
    #[must_use]
    pub fn has_columns(&self) -> bool {
        self.title.is_some()
            || self.system.is_some()
            || self.region.is_some()
            || self.model.is_some()
            || self.revision.is_some()
            || self.serial.is_some()
            || self.variant.is_some()
            || self.complete.is_some()
            || self.modified.is_some()
    }
}
