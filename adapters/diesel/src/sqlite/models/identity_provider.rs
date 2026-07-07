use diesel::prelude::*;

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::sqlite::schema::identity_provider)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub(crate) struct IdentityProvider {
    pub user_id: String,
    pub name: Option<String>,
    pub password_hash: Option<String>,
}
