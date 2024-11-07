use crate::{
    domain::{
        dtos::{
            account::Account,
            account_type::AccountTypeV2,
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::Provider,
            webhook::{AccountPropagationWebHookResponse, WebhookTrigger},
        },
        entities::{AccountRegistration, UserFetching, WebHookFetching},
    },
    use_cases::support::dispatch_webhooks,
};

use mycelium_base::{
    entities::{
        FetchManyResponseKind, FetchResponseKind, GetOrCreateResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::error;

/// Create a default account.
///
/// Default accounts are used to mirror human users. Such accounts should not be
/// flagged as `subscription`.
///
/// This function are called when a new user start into the system. The
/// account-creation method also insert a new user into the database and set the
/// default role as `default-user`.
#[tracing::instrument(name = "create_default_account", skip_all)]
pub async fn create_default_account(
    email: Email,
    account_name: String,
    user_fetching_repo: Box<&dyn UserFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch user from database
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo.get(None, Some(email), None).await? {
        FetchResponseKind::NotFound(_) => {
            return use_case_err("User not found".to_string()).as_error();
        }
        FetchResponseKind::Found(user) => user,
    };

    if let Some(Provider::Internal(_)) = user.provider() {
        if !user.is_active {
            return use_case_err("User is not active".to_string()).as_error();
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let account = match account_registration_repo
        .get_or_create_user_account(
            Account::new(account_name, user, AccountTypeV2::User),
            true,
            false,
        )
        .await?
    {
        GetOrCreateResponseKind::Created(account) => account,
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(format!("Account not created: {msg}"))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Dispatch associated webhooks
    // ? -----------------------------------------------------------------------

    let target_hooks = match webhook_fetching_repo
        .list_by_trigger(WebhookTrigger::CreateUserAccount)
        .await?
    {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => Some(records),
        _ => {
            error!("Webhook response should not be paginated");
            None
        }
    };

    let propagation_responses = match target_hooks {
        None => None,
        Some(hooks) => dispatch_webhooks(hooks, account.to_owned(), None).await,
    };

    Ok(AccountPropagationWebHookResponse {
        account,
        propagation_responses,
    })
}
