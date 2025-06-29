use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

use crate::domain::{
    actors::SystemActor,
    dtos::{
        account_type::AccountType, guest_role::Permission,
        guest_user::GuestUser, native_error_codes::NativeErrorCodes,
        profile::Profile,
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
    page_size: Option<i32>,
    skip: Option<i32>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_fetching_repo: Box<&dyn GuestUserFetching>,
) -> Result<FetchManyResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_accounts_or_tenant_wide_permission_or_error(
            tenant_id,
            Permission::Read,
        )?;

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
        AccountType::ActorAssociated { .. }
        | AccountType::TenantManager { .. }
        | AccountType::RoleAssociated { .. }
        | AccountType::Subscription { .. } => (),
        _ => {
            return use_case_err(
                "Operation restricted to subscription, tenant manager, role associated and actor associated accounts.",
            )
            .with_exp_true()
            .with_code(NativeErrorCodes::MYC00019)
            .as_error()
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Fetch guest users
    // ? -----------------------------------------------------------------------

    guest_user_fetching_repo
        .list(account_id, page_size, skip)
        .await
}
