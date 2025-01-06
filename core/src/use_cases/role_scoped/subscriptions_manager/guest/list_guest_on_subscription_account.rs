use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

use crate::domain::{
    actors::SystemActor,
    dtos::{
        account_type::AccountTypeV2, guest_user::GuestUser,
        native_error_codes::NativeErrorCodes, profile::Profile,
    },
    entities::{AccountFetching, GuestUserFetching},
};

/// List guests on subscription account
///
/// Fetch a list of the guest accounts associated with a single subscription
/// account.
#[tracing::instrument(
    name = "list_guest_on_subscription_account",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_guest_on_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_fetching_repo: Box<&dyn GuestUserFetching>,
) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_standard_accounts_access()
        .with_read_access()
        .with_roles(vec![
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Fetch the target subscription account
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo
        .get(account_id, related_accounts)
        .await?
    {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Invalid account ID: {}", id.unwrap()))
                .with_code(NativeErrorCodes::MYC00013)
                .as_error()
        }
        FetchResponseKind::Found(res) => res,
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the account is a valid subscription account
    // ? -----------------------------------------------------------------------

    match account.account_type {
        AccountTypeV2::Subscription { .. } => (),
        _ => {
            return use_case_err(
                "Operation restricted to subscription accounts.",
            )
            .as_error()
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch guest users
    // ? -----------------------------------------------------------------------

    guest_user_fetching_repo.list(account_id).await
}
