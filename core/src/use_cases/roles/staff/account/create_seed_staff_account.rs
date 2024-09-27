use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::{PasswordHash, Provider, User},
        },
        entities::{
            AccountRegistration, AccountTypeRegistration, UserRegistration,
        },
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
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
    password: String,
    user_registration_repo: Box<&dyn UserRegistration>,
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
    // ? Build local user object
    // ? -----------------------------------------------------------------------

    let user = User::new_principal_with_provider(
        None,
        email_instance,
        Provider::Internal(PasswordHash::hash_user_password(
            password.as_bytes(),
        )),
        Some(first_name),
        Some(last_name),
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the user
    // ? -----------------------------------------------------------------------

    let new_user = match user_registration_repo
        .get_or_create(user.to_owned())
        .await?
    {
        GetOrCreateResponseKind::NotCreated(user, _) => {
            return use_case_err(format!(
                "User already registered: {}",
                user.email.get_email()
            ))
            .with_code(NativeErrorCodes::MYC00002)
            .as_error()
        }
        GetOrCreateResponseKind::Created(user) => user,
    };

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
        .get_or_create(
            Account::new(account_name, new_user, account_type),
            true,
            false,
        )
        .await
}
