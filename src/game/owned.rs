//! Owned game release types.

use uuid::Uuid;

use super::Game;

/// Owned physical game release.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Owned {
    /// Unique identifier.
    pub id: Uuid,
    /// Game data.
    pub game: Game,
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
    /// Complete-in-box status.
    pub complete: bool,
    /// Hardware modification status.
    pub modified: bool,
}

/// Stored owned game release.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Row {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// Game reference.
    #[diesel(deserialize_as = crate::Uuid)]
    pub game: Uuid,
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
    /// Complete-in-box status.
    pub complete: bool,
    /// Hardware modification status.
    pub modified: bool,
}

impl Row {
    /// Resolves the game reference into a full release.
    #[must_use]
    pub fn resolve(self, game: Game) -> Owned {
        Owned {
            id: self.id,
            game,
            system: self.system,
            region: self.region,
            model: self.model,
            revision: self.revision,
            serial: self.serial,
            complete: self.complete,
            modified: self.modified,
        }
    }
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
    /// Region code.
    pub region: Option<String>,
    /// Model name.
    pub model: Option<String>,
    /// Hardware revision.
    pub revision: Option<String>,
    /// Serial number.
    pub serial: Option<String>,
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
    /// Game reference.
    #[serde(deserialize_with = "crate::patch::present")]
    pub game: Option<Uuid>,
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
        self.game.is_none()
            && self.system.is_none()
            && self.region.is_none()
            && self.model.is_none()
            && self.revision.is_none()
            && self.serial.is_none()
            && self.complete.is_none()
            && self.modified.is_none()
    }
}
