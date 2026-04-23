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

impl Order {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Asc => "ASC",
            Self::Desc => "DESC",
        }
    }
}
