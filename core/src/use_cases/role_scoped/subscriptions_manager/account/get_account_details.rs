use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::AccountFetching,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use uuid::Uuid;

/// Get details of a single account
///
/// These details could include information about guest accounts, modifications
/// and others.
#[tracing::instrument(
    name = "get_account_details",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn get_account_details(
    profile: Profile,
    tenant_id: Option<Uuid>,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = match profile.has_admin_privileges_or_error() {
        //
        // If the current account has administration privileges, the tenant id
        // is not required. Allowing the fetching of the account details without
        // tenant afinity. It should be used to fetch the account details of non
        // subscription accounts.
        //
        Ok(_) => profile,
        //
        // If the current account has no administration privileges, the tenant
        // id is required. If the tenant id is not provided, the use case should
        // return an privilege error.
        //
        Err(_) => {
            if let Some(tenant_id) = tenant_id {
                profile.on_tenant(tenant_id)
            } else {
                return execution_err(
                    "Current account has no administration privileges"
                        .to_string(),
                )
                .with_code(NativeErrorCodes::MYC00019)
                .with_exp_true()
                .as_error();
            }
        }
    }
    .with_system_accounts_access()
    .with_read_access()
    .with_roles(vec![
        SystemActor::TenantOwner,
        SystemActor::TenantManager,
        SystemActor::SubscriptionsManager,
    ])
    .get_related_account_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    account_fetching_repo
        .get(account_id, related_accounts)
        .await
}
