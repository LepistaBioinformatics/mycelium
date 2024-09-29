use crate::{
    domain::{
        dtos::{
            email::Email,
            message::Message,
            native_error_codes::NativeErrorCodes,
            token::{PasswordChangeTokenMeta, TokenMeta},
        },
        entities::{MessageSending, TokenRegistration, UserFetching},
    },
    models::AccountLifeCycle,
    settings::TEMPLATES,
};

use chrono::Local;
use mycelium_base::{
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;

#[tracing::instrument(name = "start_password_redefinition", skip_all)]
pub async fn start_password_redefinition(
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    platform_url: Option<String>,
    user_fetching_repo: Box<&dyn UserFetching>,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo
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
        TokenMeta::PasswordChange(meta) => meta,
        _ => return use_case_err("Invalid token type").as_error(),
    };

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
        .render("email/password-reset-initiated.jinja", &context)
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
            to: token_metadata.email,
            cc: None,
            subject: String::from(
                "[Password Reset Request] Email address confirmation",
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
