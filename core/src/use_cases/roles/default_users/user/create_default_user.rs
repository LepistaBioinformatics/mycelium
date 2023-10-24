use super::issue_confirmation_token_pasetor::issue_confirmation_token_pasetor;
use crate::domain::{
    dtos::{
        email::Email,
        message::Message,
        native_error_codes::NativeErrorCodes,
        session_token::TokenSecret,
        user::{PasswordHash, Provider, User},
    },
    entities::{
        MessageSending, SessionTokenRegistration, UserDeletion,
        UserRegistration,
    },
};

use clean_base::{
    entities::{DeletionResponseKind, GetOrCreateResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};
use log::error;
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
    message_sending_repo: Box<&dyn MessageSending>,
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

    let mut user = User::new_secondary_with_provider(
        None,
        email_instance.to_owned(),
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

    let pasetor_token = match issue_confirmation_token_pasetor(
        new_user_id.to_owned(),
        token_secret.to_owned(),
        None,
        token_registration_repo,
    )
    .await
    {
        Ok(res) => res,
        Err(err) => {
            // ? ---------------------------------------------------------------
            // ? Delete the user
            //
            // Delete user if the token registration process fails. This process
            // should be executed to avoid the creation of zombie users.
            //
            // ? ---------------------------------------------------------------

            if let DeletionResponseKind::NotDeleted(id, msg) =
                user_deletion_repo.delete(new_user_id).await?
            {
                error!(
                    "Unable to delete user: {}. Error: {}. Generated after: {}",
                    id.to_string(),
                    msg,
                    err
                );

                return use_case_err(format!(
                    "Unexpected error on create user: {}",
                    email_instance.to_owned().get_email()
                ))
                .as_error();
            };

            return Err(err);
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    if let Err(err) = message_sending_repo
        .send(Message {
            from: token_secret.token_email_notifier,
            to: email_instance,
            cc: None,
            subject: String::from("Action required: Confirm your email address"),
            message_head: Some(
                "You must confirm your email address to complete your registration process"
                .to_string()
            ),
            message_body: format!(
                "Use the follow activation token to validate your identity:\n{}",
                pasetor_token,
            ),
            message_footer: None,
        })
        .await
    {
        return use_case_err(format!("Unable to send email: {err}")).as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(new_user_id)
}
