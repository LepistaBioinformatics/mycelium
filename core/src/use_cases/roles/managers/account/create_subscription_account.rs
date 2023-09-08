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
            user::User,
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

use chrono::Local;
use clean_base::{
    dtos::{enums::ParentEnum, Children},
    entities::GetOrCreateResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};

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

    if !profile.is_manager {
        return use_case_err(
            "The current user has no sufficient privileges to register 
            subscription accounts."
                .to_string(),
        )
        .as_error();
    }

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

    let account = match account_registration_repo
        .get_or_create(Account {
            id: None,
            name: account_name,
            is_active: true,
            is_checked: true,
            is_archived: false,
            verbose_status: None,
            is_default: false,
            owners: Children::Records(
                [User {
                    id: None,
                    username: email_instance.to_owned().username,
                    email: email_instance,
                    first_name: Some(String::from("")),
                    last_name: Some(String::from("")),
                    is_active: true,
                    created: Local::now(),
                    updated: None,
                    account: None,
                }]
                .to_vec(),
            ),
            account_type: ParentEnum::Record(account_type),
            guest_users: None,
            created: Local::now(),
            updated: None,
        })
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
