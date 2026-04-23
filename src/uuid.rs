//! UUID newtype for SQLite BLOB storage.

use diesel::backend::Backend;
use diesel::deserialize::{self, FromSql};
use diesel::serialize::{self, Output, ToSql};
use diesel::sql_types::Binary;
use diesel::sqlite::Sqlite;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[derive(diesel::AsExpression, diesel::FromSqlRow)]
#[diesel(sql_type = Binary)]
pub struct Uuid(uuid::Uuid);

impl ToSql<Binary, Sqlite> for Uuid {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Sqlite>) -> serialize::Result {
        <[u8] as ToSql<Binary, Sqlite>>::to_sql(self.0.as_bytes(), out)
    }
}

impl FromSql<Binary, Sqlite> for Uuid {
    fn from_sql(bytes: <Sqlite as Backend>::RawValue<'_>) -> deserialize::Result<Self> {
        let bytes = <Vec<u8> as FromSql<Binary, Sqlite>>::from_sql(bytes)?;
        uuid::Uuid::from_slice(&bytes).map(Uuid).map_err(Into::into)
    }
}

impl From<uuid::Uuid> for Uuid {
    fn from(u: uuid::Uuid) -> Self {
        Uuid(u)
    }
}

impl From<Uuid> for uuid::Uuid {
    fn from(u: Uuid) -> Self {
        u.0
    }
}
