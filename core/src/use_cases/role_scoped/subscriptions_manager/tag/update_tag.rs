use crate::domain::{
    actors::SystemActor,
    dtos::{guest_role::Permission, profile::Profile, tag::Tag},
    entities::AccountTagUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "update_tag", 
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_tag(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    tag: Tag,
    tag_updating_repo: Box<&dyn AccountTagUpdating>,
) -> Result<UpdatingResponseKind<Tag>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
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
        .get_related_accounts_or_tenant_wide_permission_or_error(
            tenant_id,
            Permission::Write,
        )?;

    // ? -----------------------------------------------------------------------
    // ? Register tag
    // ? -----------------------------------------------------------------------

    tag_updating_repo.update(tag).await
}
