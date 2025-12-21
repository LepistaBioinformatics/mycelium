use crate::domain::{
    dtos::{
        account_type::AccountType, profile::Profile,
        related_accounts::RelatedAccounts,
    },
    entities::AccountDeletion,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_tenant_manager_account",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty
    ),
    skip(profile, account_deletion_repo)
)]
pub async fn delete_tenant_manager_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_deletion_repo: Box<&dyn AccountDeletion>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Delete account
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .soft_delete_account(
            account_id,
            AccountType::TenantManager { tenant_id },
            RelatedAccounts::AllowedAccounts(vec![account_id]),
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(response)
}
