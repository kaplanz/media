//! Partial update helpers.

use serde::{Deserialize, Deserializer};

/// Deserializes a present field as `Some`.
///
/// Serde maps an explicit `null` on an `Option<Option<T>>` field to the
/// outer `None`, making it indistinguishable from an absent field. This
/// wrapper keeps a present `null` as `Some(None)` so it can clear a
/// nullable column, while absent fields fall back to the struct default.
pub(crate) fn present<'de, T, D>(de: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: Deserializer<'de>,
{
    T::deserialize(de).map(Some)
}
