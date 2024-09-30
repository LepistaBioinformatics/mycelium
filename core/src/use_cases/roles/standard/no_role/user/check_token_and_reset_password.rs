use crate::{
    domain::{
        dtos::{
            email::Email, message::Message,
            native_error_codes::NativeErrorCodes,
            token::PasswordChangeTokenMeta, user::PasswordHash,
        },
        entities::{
            MessageSending, TokenInvalidation, UserFetching, UserUpdating,
        },
    },
    models::AccountLifeCycle,
    settings::TEMPLATES,
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;

#[tracing::instrument(name = "check_token_and_reset_password", skip_all)]
pub async fn check_token_and_reset_password(
    token: String,
    email: Email,
    new_password: String,
    platform_url: Option<String>,
    life_cycle_settings: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_updating_repo: Box<&dyn UserUpdating>,
    token_invalidation_repo: Box<&dyn TokenInvalidation>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let target_user = match user_fetching_repo
        .get(None, Some(email.to_owned()), None)
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
    // ? Validate token
    // ? -----------------------------------------------------------------------

    let meta = PasswordChangeTokenMeta::new(
        match target_user.id {
            Some(id) => id,
            None => {
                return use_case_err(format!(
                    "Unexpected error: User with email {email} has no id",
                    email = email.get_email()
                ))
                .as_error()
            }
        },
        email.to_owned(),
        token,
    );

    let user_id = match token_invalidation_repo
        .get_and_invalidate_password_change_token(meta.to_owned())
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Token not found or expired for user with email {}",
                email.get_email()
            ))
            .with_code(NativeErrorCodes::MYC00008)
            .with_exp_true()
            .as_error()
        }
        FetchResponseKind::Found(id) => id,
    };

    // ? -----------------------------------------------------------------------
    // ? Update user password
    // ? -----------------------------------------------------------------------

    let mut hash_password =
        PasswordHash::hash_user_password(new_password.as_bytes());

    hash_password.with_raw_password(new_password);

    if let UpdatingResponseKind::NotUpdated((code, _), msg) = user_updating_repo
        .update_password(user_id, hash_password)
        .await?
    {
        let mut error = use_case_err(format!(
            "User with id {} could not be activated: {}",
            user_id.to_string(),
            msg
        ))
        .with_exp_true();

        if let Some(c) = code {
            error = error.with_code(c);
        }

        return error.as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Build notification message
    // ? -----------------------------------------------------------------------

    let mut context = Context::new();
    context.insert("verification_code", &meta.get_token());
    context.insert(
        "support_email",
        &life_cycle_settings.support_email.get_or_error()?,
    );

    if let Some(url) = platform_url {
        context.insert("platform_url", &url);
    }

    let email_template = match TEMPLATES
        .render("email/password-reset-confirmation.jinja", &context)
    {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!(
                "Unable to render email template: {err}"
            ))
            .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify user owner
    // ? -----------------------------------------------------------------------

    if let Err(err) = message_sending_repo
        .send(Message {
            from: Email::from_string(
                life_cycle_settings.noreply_email.get_or_error()?,
            )?,
            to: email,
            cc: None,
            subject: String::from(
                "[Password Reset Success] Email address confirmation",
            ),
            message_head: None,
            message_body: email_template,
            message_footer: None,
        })
        .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    Ok(())
}
