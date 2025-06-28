use crate::domain::{
    actors::SystemActor, dtos::profile::Profile, entities::AccountTagDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tag",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn delete_tag(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    tag_id: Uuid,
    tag_deletion_repo: Box<&dyn AccountTagDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    //
    // Despite the action itself is a deletion one, user must have the
    // permission to update the guest account.
    //
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .on_account(account_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_accounts_or_tenant_wide_permission_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_deletion_repo.delete(tag_id).await
}
