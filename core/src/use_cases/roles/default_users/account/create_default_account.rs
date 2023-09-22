use crate::{
    domain::{
        dtos::account::{Account, AccountTypeEnum},
        entities::{
            AccountRegistration, AccountTypeRegistration, UserFetching,
        },
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use clean_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
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
pub async fn create_default_account(
    user_id: Uuid,
    account_name: String,
    user_fetching_repo: Box<&dyn UserFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
) -> Result<Account, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch user from database
    // ? -----------------------------------------------------------------------

    let mut user =
        match user_fetching_repo.get(Some(user_id), None, None).await? {
            FetchResponseKind::NotFound(_) => {
                return use_case_err("User not found".to_string()).as_error();
            }
            FetchResponseKind::Found(user) => user,
        };

    user.is_active = true;

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

    match account_registration_repo
        .get_or_create(
            Account::new(account_name, user, account_type),
            true,
            false,
        )
        .await?
    {
        GetOrCreateResponseKind::Created(account) => Ok(account),
        GetOrCreateResponseKind::NotCreated(_, _) => {
            use_case_err("Account not created".to_string()).as_error()
        }
    }
}
