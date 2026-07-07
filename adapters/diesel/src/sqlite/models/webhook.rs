use diesel::prelude::*;

#[derive(Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::sqlite::schema::webhook)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct WebHook {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub url: String,
    pub trigger: String,
    pub method: Option<String>,
    pub secret: Option<String>,
    pub is_active: bool,
    pub created: String,
    pub created_by: Option<String>,
    pub updated: Option<String>,
    pub updated_by: Option<String>,
}
