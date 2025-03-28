use super::guest_role::GuestRole;

use chrono::NaiveDateTime;
use diesel::prelude::*;
use uuid::Uuid;

#[derive(Identifiable, Associations, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::guest_role_children)]
#[diesel(belongs_to(GuestRole, foreign_key = parent_id))]
#[diesel(primary_key(parent_id, child_role_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct GuestRoleChildren {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub parent_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub child_role_id: Uuid,
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    pub created_by: Uuid,
    pub created: NaiveDateTime,
    pub updated: Option<NaiveDateTime>,
}
