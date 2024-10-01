use crate::domain::{
    actors::DefaultActor,
    dtos::{profile::Profile, tag::Tag},
    entities::TagRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use std::collections::HashMap;
use uuid::Uuid;

#[tracing::instrument(name = "register_tag", skip_all)]
pub async fn register_tag(
    profile: Profile,
    tag: String,
    meta: HashMap<String, String>,
    account_id: Uuid,
    tag_registration_repo: Box<&dyn TagRegistration>,
) -> Result<GetOrCreateResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.get_default_create_ids_or_error(vec![
        DefaultActor::SubscriptionManager.to_string(),
    ])?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_registration_repo
        .get_or_create(account_id, tag, meta)
        .await
}
