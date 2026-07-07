use diesel::prelude::*;
use diesel::sql_types::{Bool, Integer, Nullable, Text};

/// Mirrors the `licensed_resources` SQLite view. `permit_flags`/`deny_flags`
/// are `Array<Text>` on postgres; here they're a single JSON-array TEXT value
/// (see `sqlite::types::string_array_from_text`).
#[derive(QueryableByName)]
pub(crate) struct LicensedResourceRow {
    #[diesel(sql_type = Text)]
    pub acc_id: String,
    #[diesel(sql_type = Text)]
    pub gr_id: String,
    #[diesel(sql_type = Text)]
    pub acc_name: String,
    #[diesel(sql_type = Nullable<Text>)]
    pub tenant_id: Option<String>,
    #[diesel(sql_type = Bool)]
    pub is_acc_std: bool,
    #[diesel(sql_type = Text)]
    pub gr_slug: String,
    #[diesel(sql_type = Integer)]
    pub gr_perm: i32,
    #[diesel(sql_type = Bool)]
    pub gu_verified: bool,
    #[diesel(sql_type = Nullable<Text>)]
    pub permit_flags: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    pub deny_flags: Option<String>,
}
