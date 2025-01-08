use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::{AccountRegistration, TenantFetching},
};

use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
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
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    let tenant = match tenant_fetching_repo
        .get_tenant_owned_by_me(tenant_id, profile.get_owners_ids())
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

    let name = format!("{}/manager", tenant.tenant_string_or_error()?);
    unchecked_account.name = name.to_owned();
    unchecked_account.slug = slugify!(&name.as_str());

    account_registration_repo
        .get_or_create_tenant_management_account(unchecked_account, tenant_id)
        .await
}
