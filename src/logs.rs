//! Activity logs.

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Text;
use diesel::sqlite::Sqlite;
use uuid::Uuid;

/// Activity log.
#[derive(Clone, Copy, Debug)]
#[derive(diesel::Queryable)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Log {
    /// Unique identifier.
    #[diesel(deserialize_as = crate::Uuid)]
    pub id: Uuid,
    /// Activity kind.
    pub kind: Kind,
    /// Activity date (Unix seconds).
    pub date: i64,
}

/// Activity kind.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[derive(diesel::AsExpression, diesel::FromSqlRow)]
#[derive(strum::AsRefStr, strum::Display, strum::EnumString)]
#[derive(serde::Deserialize, serde::Serialize)]
#[derive(utoipa::ToSchema)]
#[diesel(sql_type = Text)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum Kind {
    /// Activity started.
    Start,
    /// Activity stopped.
    Stop,
    /// Activity completed.
    Done,
}

impl ToSql<Text, Sqlite> for Kind {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <str as ToSql<Text, Sqlite>>::to_sql(self.as_ref(), out)
    }
}

impl FromSql<Text, Sqlite> for Kind {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let text = <String as FromSql<Text, Sqlite>>::from_sql(bytes)?;
        text.parse().map_err(Into::into)
    }
}

/// Request body.
#[derive(Clone, Copy, Debug)]
#[derive(utoipa::ToSchema)]
#[derive(serde::Deserialize, serde::Serialize)]
pub struct Body {
    /// Activity kind.
    pub kind: Kind,
    /// Activity date (Unix seconds); defaults to now.
    pub date: Option<i64>,
}
