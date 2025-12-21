use crate::{
    domain::{
        dtos::{
            account::Account,
            account_type::AccountType,
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::{Provider, User},
            webhook::{PayloadId, WebHookTrigger},
            written_by::WrittenBy,
        },
        entities::{
            AccountRegistration, LocalMessageWrite, TenantFetching,
            UserFetching, UserRegistration, WebHookRegistration,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::{
        dispatch_notification, register_webhook_dispatching_event,
    },
};

use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
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
    name = "create_user_account",
    fields(correspondence_id = tracing::field::Empty),
    skip_all
)]
pub async fn create_user_account(
    email: Email,
    provider: Option<String>,
    account_name: String,
    config: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a user account");

    // ? -----------------------------------------------------------------------
    // ? Try to fetch user from database
    // ? -----------------------------------------------------------------------

    let (identity_is_verified, _provider) = if let Some(provider) = provider {
        (true, Some(provider))
    } else {
        (false, None)
    };

    let user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            if !identity_is_verified || _provider.is_none() {
                return use_case_err("User not found".to_string()).as_error();
            }

            register_user_with_provider(
                email.to_owned(),
                _provider.unwrap(),
                user_registration_repo,
            )
            .await?
        }
        FetchResponseKind::Found(user) => user,
    };

    if let Some(Provider::Internal(_)) = user.provider() {
        if !user.is_active {
            return use_case_err("User is not active".to_string())
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00018)
                .as_error();
        }
    }

    if !user.is_principal() {
        return use_case_err("User is not the principal".to_string())
            .with_exp_true()
            .with_code(NativeErrorCodes::MYC00018)
            .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut base_account = Account::new(
        account_name.to_owned(),
        user.clone(),
        AccountType::User,
        Some(WrittenBy::new_from_user(user.id.ok_or_else(|| {
            use_case_err("User ID not found".to_string()).with_exp_true()
        })?)),
    );

    base_account.slug = slugify!(user.email.email().as_str());

    let account = match account_registration_repo
        .get_or_create_user_account(base_account, true, false)
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

    let account_id = account.id.ok_or_else(|| {
        use_case_err("Account ID not found".to_string()).with_exp_true()
    })?;

    let (notification_response, webhook_responses) = futures::join!(
        dispatch_notification(
            vec![("account_name", account_name)],
            "email/create-user-account",
            config.to_owned(),
            email,
            None,
            message_sending_repo,
            tenant_fetching_repo,
        ),
        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountCreated,
            account.to_owned(),
            PayloadId::Uuid(account_id),
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

async fn register_user_with_provider(
    email: Email,
    provider: String,
    user_registration_repo: Box<&dyn UserRegistration>,
) -> Result<User, MappedErrors> {
    let user = User::new_principal_with_provider(
        None,
        email.to_owned(),
        Provider::External(provider),
        None,
        None,
    )?;

    match user_registration_repo
        .get_or_create(user.to_owned())
        .await?
    {
        GetOrCreateResponseKind::Created(user) => Ok(user),
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            tracing::error!("User not created: {msg}");

            use_case_err("User not created".to_string()).as_error()
        }
    }
}
