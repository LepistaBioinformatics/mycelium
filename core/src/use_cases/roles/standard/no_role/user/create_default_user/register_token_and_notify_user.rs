use crate::{
    domain::{
        dtos::{
            email::Email,
            message::Message,
            session_token::TokenSecret,
            token::{EmailConfirmationTokenMeta, TokenMeta},
        },
        entities::{MessageSending, TokenRegistration, UserDeletion},
    },
    settings::TEMPLATES,
    use_cases::roles::standard::no_role::user::delete_default_user,
};

use chrono::Local;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use tera::Context;
use uuid::Uuid;

pub(super) async fn register_token_and_notify_user(
    user_id: Uuid,
    email: Email,
    token_secret: TokenSecret,
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

    let random_number: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(6)
        .map(char::from)
        .collect();

    let meta = EmailConfirmationTokenMeta::new(
        user_id,
        email.to_owned(),
        random_number,
    );

    let token = match token_registration_repo
        .create_email_confirmation_token(
            meta,
            Local::now()
                + chrono::Duration::hours(token_secret.token_expiration),
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
        TokenMeta::EmailConfirmation(meta) => meta,
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
    context.insert("verification_code", &token_metadata.get_token());
    context
        .insert("support_email", &token_secret.support_email.get_or_error()?);

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
                token_secret.noreply_email.get_or_error()?,
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
        return use_case_err(format!("Unable to send email: {err}")).as_error();
    };

    Ok(())
}
