use crate::domain::{
    actors::ActorName,
    dtos::{account::Account, profile::Profile},
    entities::{AccountRegistration, TenantFetching},
};

use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "create_management_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, tenant_fetching_repo, account_registration_repo)
)]
pub async fn create_management_account(
    profile: Profile,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<CreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_create_or_error(vec![
            ActorName::TenantOwner.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    let tenant = match tenant_fetching_repo
        .get(tenant_id, related_accounts)
        .await?
    {
        FetchResponseKind::NotFound(msg) => {
            return use_case_err(
                msg.unwrap_or(
                    "Tenant does not exist or inaccessible for the user"
                        .to_string(),
                ),
            )
            .as_error()
        }
        FetchResponseKind::Found(tenant) => tenant,
    };

    // ? -----------------------------------------------------------------------
    // ? Register account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account =
        Account::new_tenant_management_account(String::new(), tenant_id)
            .with_id();

    unchecked_account.is_checked = true;

    unchecked_account.name = if let Some(id) = unchecked_account.id {
        format!(
            "{}/manager/{}",
            tenant.tenant_string_or_error()?,
            id.to_string()
        )
    } else {
        return use_case_err("Unable to predict account name").as_error();
    };

    account_registration_repo
        .create_subscription_account(unchecked_account, tenant_id)
        .await
}
