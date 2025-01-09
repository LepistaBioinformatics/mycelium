mod register_token_and_notify_user;

use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            user::{PasswordHash, Provider, User},
        },
        entities::{
            MessageSending, TokenRegistration, UserDeletion, UserRegistration,
        },
    },
    models::AccountLifeCycle,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use register_token_and_notify_user::register_token_and_notify_user;
use uuid::Uuid;

/// Create a new user with the default provider
///
/// This function creates a new user with the default provider. The default
/// provider is the internal provider, which uses the user's email and
/// password/provider to authenticate the user. Case the user is created with
/// the internal provider, the user is created as inactive, forcing the user to
/// confirm the email address before the user can use the system. The user
/// activation process is done by sending a confirmation email to the user.
/// If the user is created with an external provider, the user is created as
/// active.
///
#[tracing::instrument(name = "create_default_user", skip_all)]
pub async fn create_default_user(
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    provider_name: Option<String>,
    life_cycle_settings: AccountLifeCycle,
    user_registration_repo: Box<&dyn UserRegistration>,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
    user_deletion_repo: Box<&dyn UserDeletion>,
) -> Result<Uuid, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build and validate email
    //
    // Build the Email object, case an error is returned, the email is
    // possibly invalid.
    //
    // ? -----------------------------------------------------------------------

    let email_instance = Email::from_string(email.to_lowercase())?;

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

    let mut user = User::new_principal_with_provider(
        None,
        email_instance.to_owned(),
        match password.to_owned() {
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

    // ! By default new users are created as active ones. But when the user
    // ! provider is internal the user is created as inactive, forcing new users
    // ! to check their email address before they can use the system.
    if let Some(Provider::Internal(_)) = user.provider() {
        user.is_active = false;
    }

    let new_user = match user_registration_repo
        .get_or_create(user.to_owned())
        .await?
    {
        GetOrCreateResponseKind::NotCreated(user, _) => {
            if let Some(Provider::Internal(_)) = user.provider() {
                if user.is_active {
                    return use_case_err(
                        "You are trying to re-create an active user. Try to recovery your password instead"
                            .to_string(),
                    )
                    .with_code(NativeErrorCodes::MYC00002)
                    .with_exp_true()
                    .as_error();
                }

                user
            } else {
                user
            }
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
    // ? Notify internal user
    // ? -----------------------------------------------------------------------

    if let Some(Provider::Internal(_)) = user.provider() {
        register_token_and_notify_user(
            new_user_id,
            email_instance.to_owned(),
            life_cycle_settings,
            token_registration_repo,
            message_sending_repo,
            user_deletion_repo,
        )
        .await?;
    }

    // ? -----------------------------------------------------------------------
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(new_user_id)
}
