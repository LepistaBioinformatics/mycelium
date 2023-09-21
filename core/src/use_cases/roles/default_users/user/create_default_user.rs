use super::issue_confirmation_token_pasetor::issue_confirmation_token_pasetor;
use crate::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        session_token::TokenSecret,
        user::{PasswordHash, Provider, User},
    },
    entities::{
        SessionTokenRegistration, SessionTokenUpdating, UserDeletion,
        UserRegistration,
    },
};

use clean_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use uuid::Uuid;

pub async fn create_default_user(
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    provider_name: Option<String>,
    token_secret: TokenSecret,
    user_registration_repo: Box<&dyn UserRegistration>,
    user_deletion_repo: Box<&dyn UserDeletion>,
    token_registration_repo: Box<&dyn SessionTokenRegistration>,
    token_updating_repo: Box<&dyn SessionTokenUpdating>,
) -> Result<Uuid, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the Email object, case an error is returned, the email is
    // possibly invalid.
    //
    // ? -----------------------------------------------------------------------

    let email_instance = Email::from_string(email)?;

    // ? -----------------------------------------------------------------------
    // ? Build local user object
    // ? -----------------------------------------------------------------------

    if password.is_none() && provider_name.is_none() {
        return use_case_err(
            "At last one `password` or `provider-name` must contains a value"
                .to_string(),
        )
        .as_error();
    }

    let mut user = User::new_with_provider(
        None,
        email_instance,
        match password {
            Some(password) => Provider::Internal(
                PasswordHash::hash_user_password(password.as_bytes()),
            ),
            None => Provider::External(provider_name.unwrap()),
        },
        first_name,
        last_name,
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the user
    //
    // New created user should be registered as inactive user (is_active =
    // false). The activation process should occur after the user confirm the
    // email address.
    //
    // ? -----------------------------------------------------------------------

    user.is_active = false;

    let new_user = match user_registration_repo.get_or_create(user).await? {
        GetOrCreateResponseKind::NotCreated(user, _) => {
            return use_case_err(format!(
                "User already registered: {}",
                user.email.get_email()
            ))
            .with_code(NativeErrorCodes::MYC00002.as_str())
            .as_error()
        }
        GetOrCreateResponseKind::Created(user) => user,
    };

    let new_user_id = match new_user.id {
        None => {
            return use_case_err(
                "Unable to create user. Invalid user ID".to_string(),
            )
            .as_error()
        }
        Some(id) => id,
    };

    // ? -----------------------------------------------------------------------
    // ? Issue a verification token
    // ? -----------------------------------------------------------------------

    match issue_confirmation_token_pasetor(
        new_user_id.to_owned(),
        token_secret,
        None,
        token_registration_repo,
        token_updating_repo,
    )
    .await
    {
        Ok(_) => {}
        Err(err) => {
            // ? ---------------------------------------------------------------
            // ? Delete the user
            //
            // Delete user if the token registration process fails. This process
            // should be executed to avoid the creation of zombie users.
            //
            // ? ---------------------------------------------------------------

            user_deletion_repo.delete(new_user_id).await?;

            return Err(err);
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(new_user_id)
}
