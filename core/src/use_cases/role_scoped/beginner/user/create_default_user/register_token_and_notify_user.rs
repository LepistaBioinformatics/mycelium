use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            token::{EmailConfirmationTokenMeta, MultiTypeMeta},
        },
        entities::{MessageSending, TokenRegistration, UserDeletion},
    },
    models::AccountLifeCycle,
    use_cases::{
        role_scoped::beginner::user::delete_default_user,
        support::send_email_notification,
    },
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(name = "register_token_and_notify_user", skip_all)]
pub(super) async fn register_token_and_notify_user(
    user_id: Uuid,
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
    user_deletion_repo: Box<&dyn UserDeletion>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register confirmation token
    //
    // The token should be a random number with 6 decimal places. Example:
    // 096579
    //
    // ? -----------------------------------------------------------------------

    let meta = EmailConfirmationTokenMeta::new_with_random_token(
        user_id,
        email.to_owned(),
        0,
        499_999,
    );

    let expires_at = Local::now()
        + Duration::seconds(
            life_cycle_settings
                .token_expiration
                .async_get_or_error()
                .await?,
        );

    let token = match token_registration_repo
        .create_email_confirmation_token(meta.to_owned(), expires_at)
        .await
    {
        Ok(res) => match res {
            CreateResponseKind::Created(token) => token,
            CreateResponseKind::NotCreated(_, msg) => {
                // ? -----------------------------------------------------------
                // ? Delete the user
                //
                // Delete user if the token registration process fails. This
                // process should be executed to avoid the creation of zombie
                // users.
                //
                // ? -----------------------------------------------------------

                delete_default_user(user_id, user_deletion_repo.to_owned())
                    .await?;
                return use_case_err(msg).as_error();
            }
        },
        Err(err) => {
            // ? ---------------------------------------------------------------
            // ? Delete the user
            //
            // Delete user if the token registration process fails. This process
            // should be executed to avoid the creation of zombie users.
            //
            // ? ---------------------------------------------------------------

            delete_default_user(user_id, user_deletion_repo.to_owned()).await?;
            return Err(err);
        }
    };

    let token_metadata = match token.to_owned().meta {
        MultiTypeMeta::EmailConfirmation(meta) => meta,
        _ => {
            // ? ---------------------------------------------------------------
            // ? Delete the user
            // ? ---------------------------------------------------------------
            delete_default_user(user_id, user_deletion_repo.to_owned()).await?;
            return use_case_err("Invalid token type").as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    let parameters = vec![("verification_code", meta.get_token())];

    if let Err(err) = send_email_notification(
        parameters,
        "email/activation-code.jinja",
        life_cycle_settings,
        token_metadata.email,
        None,
        String::from("Please confirm your email address"),
        message_sending_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    Ok(())
}
