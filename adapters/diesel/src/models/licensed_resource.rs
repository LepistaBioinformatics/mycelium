use diesel::{
    prelude::*,
    sql_types::{Array, Bool, Integer, Nullable, Text},
};
use uuid::Uuid;

#[derive(QueryableByName)]
pub(crate) struct LicensedResourceRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub acc_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub gr_id: Uuid,
    #[diesel(sql_type = Text)]
    pub acc_name: String,
    #[diesel(sql_type = Nullable<diesel::sql_types::Uuid>)]
    pub tenant_id: Option<Uuid>,
    #[diesel(sql_type = Bool)]
    pub is_acc_std: bool,
    #[diesel(sql_type = Text)]
    pub gr_slug: String,
    #[diesel(sql_type = Integer)]
    pub gr_perm: i32,
    #[diesel(sql_type = Bool)]
    pub gu_verified: bool,
    #[diesel(sql_type = Nullable<Array<Text>>)]
    pub permit_flags: Option<Vec<String>>,
    #[diesel(sql_type = Nullable<Array<Text>>)]
    pub deny_flags: Option<Vec<String>>,
}
