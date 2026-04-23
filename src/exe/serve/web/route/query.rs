//! Shared query parameter types.

/// Sort direction.
#[derive(Clone, Copy, Debug, Default)]
#[derive(serde::Deserialize)]
#[derive(utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    /// Ascending.
    Asc,
    /// Descending.
    #[default]
    Desc,
}
