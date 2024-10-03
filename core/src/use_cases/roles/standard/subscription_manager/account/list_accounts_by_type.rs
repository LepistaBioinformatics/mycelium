use crate::domain::{
    actors::ActorName,
    dtos::{account::Account, account_type::AccountTypeV2, profile::Profile},
    entities::AccountFetching,
    utils::try_as_uuid,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// List account given an account-type
///
/// Get a list of available accounts given the AccountTypeEnum.
#[tracing::instrument(
    name = "list_accounts_by_type",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_accounts_by_type(
    profile: Profile,
    tenant_id: Uuid,
    term: Option<String>,
    is_owner_active: Option<bool>,
    is_account_active: Option<bool>,
    is_account_checked: Option<bool>,
    is_account_archived: Option<bool>,
    is_subscription: Option<bool>,
    tag_value: Option<String>,
    page_size: Option<i32>,
    skip: Option<i32>,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_view_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? List accounts
    // ? -----------------------------------------------------------------------

    let (updated_term, account_id) = {
        if let Some(i) = term {
            match try_as_uuid(&i) {
                Ok(id) => (None, Some(id)),
                Err(_) => (Some(i), None),
            }
        } else {
            (None, None)
        }
    };

    let (updated_tag, tag_id) = {
        if let Some(i) = tag_value {
            match try_as_uuid(&i) {
                Ok(id) => (None, Some(id)),
                Err(_) => (Some(i), None),
            }
        } else {
            (None, None)
        }
    };

    account_fetching_repo
        .list(
            related_accounts,
            updated_term,
            is_owner_active,
            is_account_active,
            is_account_checked,
            is_account_archived,
            tag_id,
            updated_tag,
            account_id,
            if let Some(true) = is_subscription {
                Some(AccountTypeV2::Subscription { tenant_id })
            } else {
                None
            },
            Some(is_subscription.unwrap_or(false)),
            page_size,
            skip,
        )
        .await
}
