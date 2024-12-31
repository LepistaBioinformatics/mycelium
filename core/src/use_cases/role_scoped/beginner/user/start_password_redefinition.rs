use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            token::{MultiTypeMeta, PasswordChangeTokenMeta},
        },
        entities::{MessageSending, TokenRegistration, UserFetching},
    },
    models::AccountLifeCycle,
    use_cases::support::send_email_notification,
};

use chrono::Local;
use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

#[tracing::instrument(name = "start_password_redefinition", skip_all)]
pub async fn start_password_redefinition(
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "User not found: {}",
                email.get_email()
            ))
            .with_code(NativeErrorCodes::MYC00009)
            .with_exp_true()
            .as_error()
        }
        FetchResponseKind::Found(user) => user,
    };

    // ? -----------------------------------------------------------------------
    // ? Register password redefinition token
    //
    // The token should be a random number with 6 decimal places. Example:
    // 096579
    //
    // ? -----------------------------------------------------------------------

    let user_id = match user.id {
        Some(id) => id,
        None => {
            return use_case_err(format!(
                "Unexpected error: User with email {email} has no id",
                email = email.get_email()
            ))
            .as_error()
        }
    };

    let meta = PasswordChangeTokenMeta::new_with_random_token(
        user_id,
        email.to_owned(),
        500_000,
        999_999,
    );

    let token = match token_registration_repo
        .create_password_change_token(
            meta.to_owned(),
            Local::now()
                + chrono::Duration::seconds(
                    life_cycle_settings.token_expiration,
                ),
        )
        .await
    {
        Ok(res) => match res {
            CreateResponseKind::Created(token) => token,
            CreateResponseKind::NotCreated(_, msg) => {
                return use_case_err(msg).as_error()
            }
        },
        Err(err) => return Err(err),
    };

    let token_metadata = match token.to_owned().meta {
        MultiTypeMeta::PasswordChange(meta) => meta,
        _ => return use_case_err("Invalid token type").as_error(),
    };

    // ? -----------------------------------------------------------------------
    // ? Notify user owner
    // ? -----------------------------------------------------------------------

    if let Err(err) = send_email_notification(
        vec![("verification_code", meta.get_token())],
        "email/password-reset-initiated.jinja",
        life_cycle_settings,
        token_metadata.email,
        None,
        String::from("Password Reset Request"),
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
