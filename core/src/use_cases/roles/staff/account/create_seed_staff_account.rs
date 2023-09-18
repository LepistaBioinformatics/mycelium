use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            email::Email,
            user::User,
        },
        entities::{AccountRegistration, AccountTypeRegistration},
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use chrono::Local;
use clean_base::{
    dtos::{enums::ParentEnum, Children},
    entities::GetOrCreateResponseKind,
    utils::errors::MappedErrors,
};

/// Create a seed staff account.
///
/// Seed staff accounts should be created over the system first initialization.
/// The seed staff will be create other users.
///
/// WARNING:
/// --------
///
/// Given the possibility to create seed staff accounts without profile
/// checking, this function could not be exposed through API ports.
pub async fn create_seed_staff_account(
    email: String,
    account_name: String,
    first_name: String,
    last_name: String,
    //user_registration_repo: Box<&dyn UserRegistration>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
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
        AccountTypeEnum::Staff,
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

    account_registration_repo
        .get_or_create(Account {
            id: None,
            name: account_name,
            is_active: true,
            is_checked: false,
            is_archived: false,
            verbose_status: None,
            is_default: false,
            owners: Children::Records(
                [User {
                    id: None,
                    username: email_instance.to_owned().username,
                    email: email_instance,
                    first_name: Some(first_name),
                    last_name: Some(last_name),
                    provider: None,
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
        .await
}
