//! Video game.

use uuid::Uuid;

/// Video game.
#[derive(Clone, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Game {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// TGDB ID.
    pub tgdb: Option<i64>,
    /// Title.
    pub title: String,
    /// Platform.
    pub system: Option<String>,
    /// Owned copy count.
    pub owned: i64,
    /// Rating (1-5).
    pub rated: Option<i64>,
}

impl Game {
    pub const KIND: crate::Kind = crate::Kind::Game;
}

/// Request body.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// TGDB ID.
    pub tgdb: Option<i64>,
    /// Title.
    pub title: String,
    /// Platform.
    pub system: Option<String>,
    /// Owned copy count.
    pub owned: Option<i64>,
    /// Rating (1-5).
    pub rated: Option<i64>,
}
