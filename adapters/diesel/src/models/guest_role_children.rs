use super::guest_role::GuestRole;

use diesel::prelude::*;

#[derive(Identifiable, Associations, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::guest_role_children)]
#[diesel(belongs_to(GuestRole, foreign_key = parent_id))]
#[diesel(primary_key(parent_id, child_role_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuestRoleChildren {
    pub parent_id: String,
    pub child_role_id: String,
}
