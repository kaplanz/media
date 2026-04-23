//! Media types.

use crate::book::Book;
use crate::film::Film;
use crate::game::Game;
use crate::link::Link;
use crate::show::Show;

/// Media kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(strum::AsRefStr, strum::Display)]
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Kind {
    /// Reading item.
    Book,
    /// Watched film.
    Film,
    /// Video game.
    Game,
    /// Web bookmark.
    Link,
    /// Television show.
    Show,
}

/// Media item.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "item", rename_all = "lowercase")]
pub enum Item {
    /// Reading item.
    Book(Book),
    /// Watched film.
    Film(Film),
    /// Video game.
    Game(Game),
    /// Web bookmark.
    Link(Link),
    /// Television show.
    Show(Show),
}

impl Item {
    /// Returns the kind discriminant for this item.
    pub fn kind(&self) -> Kind {
        match self {
            Item::Book(_) => Kind::Book,
            Item::Film(_) => Kind::Film,
            Item::Game(_) => Kind::Game,
            Item::Link(_) => Kind::Link,
            Item::Show(_) => Kind::Show,
        }
    }
}

/// Item metadata.
#[derive(Clone, Copy, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Meta {
    /// Created timestamp (Unix seconds).
    pub created: i64,
    /// Updated timestamp (Unix seconds).
    pub updated: i64,
}

/// Media record.
#[derive(Clone, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Record<T: utoipa::ToSchema> {
    /// Item data with kind discriminant.
    #[serde(flatten)]
    #[schema(inline)]
    pub item: T,
    /// Item metadata.
    #[schema(inline)]
    pub meta: Meta,
    /// Applied tags.
    pub tags: Vec<String>,
}
