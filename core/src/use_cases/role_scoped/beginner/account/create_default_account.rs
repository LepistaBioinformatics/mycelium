use crate::{
    domain::{
        dtos::{
            account::Account, account_type::AccountType, email::Email,
            native_error_codes::NativeErrorCodes, user::Provider,
            webhook::WebHookTrigger,
        },
        entities::{
            AccountRegistration, MessageSending, UserFetching,
            WebHookRegistration,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::{
        register_webhook_dispatching_event, send_email_notification,
    },
};

use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Create a default account.
///
/// Default accounts are used to mirror human users. Such accounts should not be
/// flagged as `subscription`.
///
/// This function are called when a new user start into the system. The
/// account-creation method also insert a new user into the database and set the
/// default role as `default-user`.
#[tracing::instrument(
    name = "create_default_account",
    fields(correspondence_id = tracing::field::Empty),
    skip_all
)]
pub async fn create_default_account(
    email: Email,
    account_name: String,
    config: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", &Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a user account");

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

    tracing::trace!("Dispatching side effects");

    let (notification_response, webhook_responses) = futures::join!(
        send_email_notification(
            vec![("account_name", account_name)],
            "email/create-user-account",
            config.to_owned(),
            email,
            None,
            message_sending_repo,
        ),
        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountCreated,
            account.to_owned(),
            webhook_registration_repo,
        )
    );

    if let Err(err) = notification_response {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    }

    if let Err(err) = webhook_responses {
        return use_case_err(format!("Unable to register webhook: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    tracing::trace!("Side effects dispatched");

    // ? -----------------------------------------------------------------------
    // ? Return the webhook responses
    // ? -----------------------------------------------------------------------

    Ok(account)
}
