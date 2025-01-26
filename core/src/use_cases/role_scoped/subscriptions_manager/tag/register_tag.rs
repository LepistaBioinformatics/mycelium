use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, tag::Tag},
    entities::AccountTagRegistration,
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
    account_id: Uuid,
    tag: String,
    meta: HashMap<String, String>,
    tag_registration_repo: Box<&dyn AccountTagRegistration>,
) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_ids_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_registration_repo
        .get_or_create(account_id, tag, meta)
        .await
}
