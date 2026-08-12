use diesel::prelude::*;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::instance_settings)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct InstanceSettingsRow {
    pub key: String,
    pub value: String,
    pub created_by: Option<String>,
    pub updated_by: Option<String>,
    pub created: String,
    pub updated: Option<String>,
}
