//! Media collection types.

pub mod book;
pub mod film;
pub mod game;
pub mod games;
pub mod link;
pub mod show;

mod item;
mod patch;
mod uuid;

pub use self::item::{Item, Kind, Meta, Record};
pub use self::uuid::Uuid;
