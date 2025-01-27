use crate::{
    domain::{
        dtos::{
            account::Account,
            account_type::AccountType,
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::Provider,
            webhook::{WebHookPropagationResponse, WebHookTrigger},
        },
        entities::{
            AccountRegistration, MessageSending, UserFetching, WebHookFetching,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::{dispatch_webhooks, send_email_notification},
};

use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

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
    config: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<WebHookPropagationResponse<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch user from database
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
        .await?
    {
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
            Account::new(account_name.to_owned(), user, AccountType::User),
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
    // ? Perform finishing operations
    // ? -----------------------------------------------------------------------

    let (notification_response, webhook_responses) = futures::join!(
        send_email_notification(
            vec![("account_name", account_name)],
            "email/create-user-account",
            config.to_owned(),
            email,
            None,
            message_sending_repo,
        ),
        dispatch_webhooks(
            WebHookTrigger::UserAccountCreated,
            account.to_owned(),
            config,
            webhook_fetching_repo,
        )
    );

    if let Err(err) = notification_response {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Return the webhook responses
    // ? -----------------------------------------------------------------------

    Ok(webhook_responses)
}
