use crate::{
    domain::{
        dtos::{
            email::Email,
            message::Message,
            native_error_codes::NativeErrorCodes,
            token::{EmailConfirmationTokenMeta, MultiTypeMeta},
        },
        entities::{MessageSending, TokenRegistration, UserDeletion},
    },
    models::AccountLifeCycle,
    settings::TEMPLATES,
    use_cases::roles::standard::no_role::user::delete_default_user,
};

use chrono::Local;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;
use uuid::Uuid;

#[tracing::instrument(name = "register_token_and_notify_user", skip_all)]
pub(super) async fn register_token_and_notify_user(
    user_id: Uuid,
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    platform_url: Option<String>,
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

    let token = match token_registration_repo
        .create_email_confirmation_token(
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
        .render("email/activation-code.jinja", &context)
    {
        Ok(res) => res,
        Err(err) => {
            delete_default_user(user_id, user_deletion_repo.to_owned()).await?;
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
                "[Email Validation] Please confirm your email address",
            ),
            message_head: None,
            message_body: email_template,
            message_footer: None,
        })
        .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00009)
            .as_error();
    };

    Ok(())
}
