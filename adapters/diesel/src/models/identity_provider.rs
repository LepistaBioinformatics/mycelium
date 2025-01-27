use diesel::prelude::*;

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::identity_provider)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct IdentityProvider {
    pub user_id: String,
    pub name: Option<String>,
    pub password_hash: Option<String>,
}
