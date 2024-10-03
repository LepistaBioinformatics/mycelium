use crate::domain::{
    actors::ActorName,
    dtos::{account::Account, profile::Profile},
    entities::AccountFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

/// Get details of a single account
///
/// These details could include information about guest accounts, modifications
/// and others.
#[tracing::instrument(
    name = "get_account_details",
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn get_account_details(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
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
    // ? Fetch account
    // ? -----------------------------------------------------------------------

    account_fetching_repo
        .get(account_id, related_accounts)
        .await
}
