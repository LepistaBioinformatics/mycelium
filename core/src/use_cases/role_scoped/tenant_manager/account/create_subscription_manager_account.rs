use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::AccountRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Create a subscription manager account
///
/// The subscription manager account should be tenant-scoped and use the
/// AccountType::RoleAssociated actor associated option.
///
#[tracing::instrument(
    name = "create_subscription_manager_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, account_registration_repo)
)]
pub async fn create_subscription_manager_account(
    profile: Profile,
    tenant_id: Uuid,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    unimplemented!()
}
