//! Media kind types.

pub mod book;
pub mod film;
pub mod game;
pub mod link;
pub mod show;

/// The kind of a media item.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "type", content = "item", rename_all = "lowercase")]
pub enum Kind {
    /// Reading item.
    Book(book::Book),
    /// Watched film.
    Film(film::Film),
    /// Video game.
    Game(game::Game),
    /// Web bookmark.
    Link(link::Link),
    /// Television show.
    Show(show::Show),
}

/// Temporal metadata shared across all media types.
#[derive(Clone, Copy, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Meta {
    /// Creation timestamp (Unix seconds).
    pub created: i64,
    /// Last updated timestamp (Unix seconds).
    pub updated: i64,
}

/// A media record paired with its metadata and tags.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Record {
    /// Resource data.
    #[serde(flatten)]
    pub item: Kind,
    /// Temporal metadata.
    pub meta: Meta,
    /// Labels applied to this item.
    pub tags: Vec<String>,
}
