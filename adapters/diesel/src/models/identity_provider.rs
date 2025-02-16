use diesel::prelude::*;
use uuid::Uuid;

#[derive(Debug, Queryable, Insertable, Selectable)]
#[diesel(table_name = crate::schema::identity_provider)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct IdentityProvider {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub user_id: Uuid,
    pub name: Option<String>,
    pub password_hash: Option<String>,
}
