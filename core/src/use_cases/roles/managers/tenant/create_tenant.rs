use crate::domain::{
    dtos::profile::Profile,
    entities::{TenantRegistration, UserFetching},
};

use uuid::Uuid;

pub async fn create_tenant(
    profile: Profile,
    tenant_owner_id: Uuid,
    user_fetching_repo: Box<&dyn UserFetching>,
    tenant_registration_repo: Box<&dyn TenantRegistration>,
) {
    unimplemented!()
}
