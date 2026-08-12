use diesel::prelude::*;

#[derive(Identifiable, Clone, Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::tenant)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct Tenant {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub meta: Option<String>,
    /// JSON-array-of-statuses, serialized as a single TEXT value (SQLite has
    /// no native array type). See `sqlite::types::{json_array_to_text,
    /// json_array_from_text}`.
    pub status: Option<String>,
    pub created: String,
    pub updated: Option<String>,
    pub encrypted_dek: Option<String>,
    pub kek_version: i32,
}
