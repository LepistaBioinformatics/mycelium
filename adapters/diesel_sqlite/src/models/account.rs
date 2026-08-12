use diesel::prelude::*;

/// SQLite mirror of the postgres `models::account::Account`. Columns that are
/// `Uuid`/`Jsonb`/`Timestamptz` on postgres are `Text` here (see
/// `sqlite::types` for the encode/decode helpers).
#[derive(Identifiable, Clone, Debug, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::account)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct Account {
    pub id: String,
    pub name: String,
    pub created: String,
    pub created_by: Option<String>,
    pub updated: Option<String>,
    pub updated_by: Option<String>,
    pub is_active: bool,
    pub is_checked: bool,
    pub is_archived: bool,
    pub is_deleted: bool,
    pub is_default: bool,
    pub slug: String,
    pub account_type: String,
    pub tenant_id: Option<String>,
    pub meta: Option<String>,
}
