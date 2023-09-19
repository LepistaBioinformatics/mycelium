use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            email::Email,
            user::{PasswordHash, Provider, User},
        },
        entities::{AccountRegistration, AccountTypeRegistration},
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use clean_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};

/// Create a default account.
///
/// Default accounts are used to mirror human users. Such accounts should not be
/// flagged as `subscription`.
///
/// This function are called when a new user start into the system. The
/// account-creation method also insert a new user into the database and set the
/// default role as `default-user`.
pub async fn create_default_account(
    email: String,
    account_name: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
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
        AccountTypeEnum::Standard,
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
        .get_or_create(Account::new(
            account_name,
            User::new_with_provider(
                None,
                email_instance,
                match password {
                    Some(password) => Provider::Internal(
                        PasswordHash::hash_user_password(password.as_bytes()),
                    ),
                    None => Provider::External,
                },
                first_name,
                last_name,
            )?,
            account_type,
        ))
        .await
}
