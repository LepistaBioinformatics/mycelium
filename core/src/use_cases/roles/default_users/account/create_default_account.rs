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
        .get_or_create(Account {
            id: None,
            name: account_name,
            is_active: true,
            is_checked: false,
            is_archived: false,
            verbose_status: None,
            //owner: ParentEnum::Record(User {
            //    id: None,
            //    username: email_instance.to_owned().username,
            //    email: email_instance,
            //    first_name,
            //    last_name,
            //    is_active: true,
            //    created: Local::now(),
            //    updated: None,
            //}),
            owners: Children::Records(
                [User {
                    id: None,
                    username: email_instance.to_owned().username,
                    email: email_instance,
                    first_name,
                    last_name,
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
