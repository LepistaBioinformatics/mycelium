use crate::domain::{
    actors::SystemActor,
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
    account_type: Option<AccountTypeV2>,
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
        .with_standard_accounts_access()
        .with_read_access()
        .with_roles(vec![
            SystemActor::TenantOwner,
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()?;

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
            account_type,
            page_size,
            skip,
        )
        .await
}
