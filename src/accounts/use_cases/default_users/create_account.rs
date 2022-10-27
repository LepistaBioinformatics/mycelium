use crate::{
    domain::{
        dtos::{
            account::AccountDTO, email::EmailDTO, enums::ParentEnum,
            user::UserDTO,
        },
        entities::{
            default_users::{
                account_registration::AccountRegistration,
                user_registration::UserRegistration,
            },
            manager::account_type_registration::AccountTypeRegistration,
            shared::default_responses::GetOrCreateResponse,
        },
        utils::errors::MappedErrors,
    },
    use_cases::shared::get_or_create_default_account_type::get_or_create_default_account_type,
};

/// This function are called when a new user start into the system. The
/// account-creation method also insert a new user into the database and set
/// the default role as `default-user`.
pub async fn create_account(
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    user_registration_repo: Box<&dyn UserRegistration>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponse<AccountDTO>, MappedErrors> {
    // ? ----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the EmailDTO object, case an error is returned, the email is
    // possibly invalid. ?
    // ? ----------------------------------------------------------------------

    let email_instance = match EmailDTO::from_string(email) {
        Err(err) => return Err(err),
        Ok(res) => res,
    };

    // ? ----------------------------------------------------------------------
    // ? Fetch account type
    //
    // Get or create the default account-type.
    // ? ----------------------------------------------------------------------

    let account_type = match get_or_create_default_account_type(
        None,
        None,
        account_type_registration_repo,
    )
    .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            GetOrCreateResponse::NotCreated(account_type, _) => account_type,
            GetOrCreateResponse::Created(account_type) => account_type,
        },
    };

    // ? ----------------------------------------------------------------------
    // ? Check and register user
    //
    // Try to register user into database. Case use was previously registered,
    // return a left response. Usually this is the same response of the user
    // registration action.
    // ? ----------------------------------------------------------------------

    let user = match user_registration_repo
        .get_or_create(UserDTO {
            id: None,
            username: email_instance.to_owned().username,
            email: email_instance,
            first_name,
            last_name,
        })
        .await
    {
        Err(err) => return Err(err),
        Ok(res) => match res {
            GetOrCreateResponse::NotCreated(user, msg) => {
                return Err(MappedErrors::new(
                    format!(
                        "Unexpected error on persist user ({}): {}",
                        user.username,
                        msg.unwrap(),
                    ),
                    Some(true),
                    None,
                ))
            }
            GetOrCreateResponse::Created(user) => user,
        },
    };

    // ? ----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? ----------------------------------------------------------------------

    account_registration_repo
        .get_or_create(AccountDTO {
            id: None,
            owner: ParentEnum::Record(user),
            account_type: ParentEnum::Record(account_type),
            guest_users: None,
        })
        .await
}
