use crate::{
    domain::{
        dtos::{
            account::AccountDTO, email::EmailDTO, profile::ProfileDTO,
            user::UserDTO,
        },
        entities::{
            default_users::user_registration::UserRegistration,
            manager::account_type_registration::AccountTypeRegistration,
            shared::account::account_registration::AccountRegistration,
        },
    },
    use_cases::shared::account_type::get_or_create_default_account_types::{
        get_or_create_default_account_types, AccountTypeEnum,
    },
};

use agrobase::{
    dtos::enums::ParentEnum,
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use chrono::Local;

/// Create an account flagged as staff.
///
/// Staff accounts should perform management action on the system.
pub async fn create_staff_account(
    profile: ProfileDTO,
    email: String,
    account_name: String,
    user_registration_repo: Box<&dyn UserRegistration>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<AccountDTO>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    //
    // Only managers or staff users should perform such action.
    // ? -----------------------------------------------------------------------

    if (!profile.is_manager) || (!profile.is_staff) {
        return Err(use_case_err(
            "The current user has no sufficient privileges to register 
            staff accounts."
                .to_string(),
            Some(true),
            None,
        ));
    }

    // ? -----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the EmailDTO object, case an error is returned, the email is
    // possibly invalid.
    // ? -----------------------------------------------------------------------

    let email_instance = match EmailDTO::from_string(email) {
        Err(err) => return Err(err),
        Ok(res) => res,
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
    .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(account_type, _) => {
                account_type
            }
            GetOrCreateResponseKind::Created(account_type) => account_type,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Check and register user
    //
    // Try to register user into database. Case use was previously registered,
    // return a left response. Usually this is the same response of the user
    // registration action.
    // ? -----------------------------------------------------------------------

    let user = match user_registration_repo
        .get_or_create(UserDTO {
            id: None,
            username: email_instance.to_owned().username,
            email: email_instance,
            first_name: None,
            last_name: None,
            is_active: true,
            created: Local::now(),
            updated: None,
        })
        .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            GetOrCreateResponseKind::NotCreated(user, msg) => {
                return Err(use_case_err(
                    format!(
                        "Unexpected error on persist user ({}): {}",
                        user.username, msg,
                    ),
                    Some(true),
                    None,
                ))
            }
            GetOrCreateResponseKind::Created(user) => user,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    account_registration_repo
        .get_or_create(AccountDTO {
            id: None,
            name: account_name,
            is_active: true,
            is_checked: false,
            owner: ParentEnum::Record(user),
            account_type: ParentEnum::Record(account_type),
            guest_users: None,
            created: Local::now(),
            updated: None,
        })
        .await
}
