use crate::domain::{
    actors::ActorName,
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
    tag: String,
    meta: HashMap<String, String>,
    tenant_id: Uuid,
    tag_registration_repo: Box<&dyn TenantTagRegistration>,
) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![
        ActorName::TenantOwner.to_string(),
        ActorName::TenantManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_registration_repo
        .get_or_create(tenant_id, tag, meta)
        .await
}
