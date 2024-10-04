use crate::domain::{
    actors::ActorName, dtos::account::Account, entities::AccountRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Try to create or fetch a default account.
///
/// This method are called when a new user start into the system. This method
/// creates a new account flagged as default based on the given account type.
/// Different account types should be connected with different default accounts.
///
/// Default accounts given specific accesses to the user. For example, a default
/// user should be able to view example data. Staff user should be able to
/// create new users and so on.
#[tracing::instrument(
    name = "get_or_create_role_related_account",
    skip(account_registration_repo)
)]
pub(crate) async fn get_or_create_role_related_account(
    tenant_id: Uuid,
    role_id: Uuid,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize default account
    // ? -----------------------------------------------------------------------

    let mut unchecked_account = Account::new_role_related_account(
        format!("default-subscription-for-role-{}", role_id.to_string()),
        tenant_id,
        role_id,
        ActorName::CustomRole(role_id.to_string()),
    );

    unchecked_account.is_checked = true;
    unchecked_account.is_default = true;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    // ? -----------------------------------------------------------------------

    account_registration_repo
        .get_or_create_user_account(unchecked_account, false, true)
        .await
}
