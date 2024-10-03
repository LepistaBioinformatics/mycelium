use super::propagate_subscription_account::propagate_subscription_account;
use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            account::Account,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            webhook::{AccountPropagationWebHookResponse, HookTarget},
        },
        entities::{AccountRegistration, WebHookFetching},
    },
    use_cases::roles::shared::webhook::default_actions::WebHookDefaultAction,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Create an account flagged as subscription.
///
/// Subscription accounts represents results centering accounts.
#[tracing::instrument(
    name = "create_subscription_account",
    fields(account_id = %profile.acc_id),
    skip_all
)]
pub async fn create_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    bearer_token: String,
    account_name: String,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .get_default_create_ids_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account =
        Account::new_subscription_account(account_name, tenant_id);

    unchecked_account.is_checked = true;

    let account = match account_registration_repo
        .get_or_create(unchecked_account, false, true)
        .await?
    {
        GetOrCreateResponseKind::NotCreated(account, msg) => {
            return use_case_err(format!("({}): {}", account.name, msg))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        }
        GetOrCreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    propagate_subscription_account(
        profile,
        tenant_id,
        bearer_token,
        account,
        WebHookDefaultAction::CreateSubscriptionAccount,
        HookTarget::Account,
        webhook_fetching_repo,
    )
    .await
}
