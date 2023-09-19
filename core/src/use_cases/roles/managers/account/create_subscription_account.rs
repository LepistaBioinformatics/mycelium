use super::propagate_subscription_account::{
    propagate_subscription_account, PropagationResponse,
};
use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            user::{PasswordHash, Provider, User},
            webhook::HookTarget,
        },
        entities::{
            AccountRegistration, AccountTypeRegistration, WebHookFetching,
        },
    },
    use_cases::roles::{
        managers::webhook::WebHookDefaultAction,
        shared::account_type::get_or_create_default_account_types,
    },
};

use clean_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Create an account flagged as subscription.
///
/// Subscription accounts represents results centering accounts.
pub async fn create_subscription_account(
    profile: Profile,
    email: String,
    account_name: String,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<PropagationResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the Email object, case an error is returned, the email is
    // possibly invalid.
    // ? -----------------------------------------------------------------------

    let email_instance = Email::from_string(email)?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account type
    //
    // Get or create the default account-type.
    // ? -----------------------------------------------------------------------

    let account_type = match get_or_create_default_account_types(
        AccountTypeEnum::Subscription,
        None,
        None,
        account_type_registration_repo,
    )
    .await?
    {
        GetOrCreateResponseKind::NotCreated(account_type, _) => account_type,
        GetOrCreateResponseKind::Created(account_type) => account_type,
    };

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account = Account::new(
        account_name,
        User::new_with_provider(
            None,
            email_instance,
            Provider::Internal(PasswordHash::hash_user_password(
                Uuid::new_v4().as_bytes(),
            )),
            Some(String::from("")),
            Some(String::from("")),
        )?,
        account_type,
    );

    unchecked_account.is_checked = true;

    let account = match account_registration_repo
        .get_or_create(unchecked_account)
        .await?
    {
        GetOrCreateResponseKind::NotCreated(account, msg) => {
            return use_case_err(format!("({}): {}", account.name, msg))
                .with_code(NativeErrorCodes::MYC00003.as_str())
                .as_error()
        }
        GetOrCreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    propagate_subscription_account(
        profile,
        account,
        WebHookDefaultAction::CreateSubscriptionAccount,
        HookTarget::Account,
        webhook_fetching_repo,
    )
    .await
}
