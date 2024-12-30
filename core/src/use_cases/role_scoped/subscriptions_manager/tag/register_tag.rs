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
    tag: String,
    meta: HashMap<String, String>,
    account_id: Uuid,
    tag_registration_repo: Box<&dyn AccountTagRegistration>,
) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![
        SystemActor::TenantOwner.to_string(),
        SystemActor::TenantManager.to_string(),
        SystemActor::SubscriptionsManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_registration_repo
        .get_or_create(account_id, tag, meta)
        .await
}
