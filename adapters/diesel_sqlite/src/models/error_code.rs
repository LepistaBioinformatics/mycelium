use diesel::prelude::*;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::error_code)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct ErrorCode {
    pub code: i32,
    pub prefix: String,
    pub message: String,
    pub details: Option<String>,
    pub is_internal: bool,
    pub is_native: bool,
}
