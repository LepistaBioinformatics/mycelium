use diesel::prelude::*;
use uuid::Uuid;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = crate::schema::federation_grant_translation)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub(crate) struct TranslationRow {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub issuer_tenant_id: Uuid,
    pub issuer_role: String,
    pub issuer_perm: i32,
    pub local_account_id: Uuid,
    pub local_guest_role_id: Uuid,
    pub local_perm: i32,
    pub enabled: bool,
}
