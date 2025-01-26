use diesel::prelude::*;

#[derive(Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::guest_user_on_account)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuestUserOnAccount {
    pub guest_user_id: String,
    pub account_id: String,
}
