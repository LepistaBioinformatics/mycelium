use crate::domain::{
    actors::ActorName,
    dtos::{account::Account, profile::Profile},
    entities::{AccountFetching, AccountUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
use uuid::Uuid;

#[tracing::instrument(
    name = "update_account_name_and_flags",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_account_name_and_flags(
    profile: Profile,
    account_id: Uuid,
    tenant_id: Uuid,
    name: Option<String>,
    is_active: Option<bool>,
    is_checked: Option<bool>,
    is_archived: Option<bool>,
    is_default: Option<bool>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    account_updating_repo: Box<&dyn AccountUpdating>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionsManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    let mut account = match account_fetching_repo
        .get(account_id, related_accounts)
        .await?
    {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Account with id {} not found",
                id.unwrap_or_default()
            ))
            .as_error();
        }
    };

    if let Some(name) = name {
        account.name = name.to_owned();
        account.slug = slugify!(&name.as_str());
    }

    if let Some(is_active) = is_active {
        account.is_active = is_active;
    }

    if let Some(is_checked) = is_checked {
        account.is_checked = is_checked;
    }

    if let Some(is_archived) = is_archived {
        account.is_archived = is_archived;
    }

    if let Some(is_default) = is_default {
        account.is_default = is_default;
    }

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    account_updating_repo.update(account).await
}
