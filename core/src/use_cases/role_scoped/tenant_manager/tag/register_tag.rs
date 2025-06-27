use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, tag::Tag},
    entities::TenantTagRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tracing::instrument(
    name = "register_tag",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn register_tag(
    profile: Profile,
    tenant_id: Uuid,
    tag: String,
    meta: HashMap<String, String>,
    tag_registration_repo: Box<&dyn TenantTagRegistration>,
) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_related_accounts_or_tenant_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_registration_repo
        .get_or_create(tenant_id, tag, meta)
        .await
}
