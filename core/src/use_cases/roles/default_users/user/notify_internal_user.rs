use super::issue_confirmation_token_pasetor::issue_confirmation_token_pasetor;
use crate::domain::{
    dtos::{email::Email, message::Message, session_token::TokenSecret},
    entities::{MessageSending, SessionTokenRegistration, UserDeletion},
};

use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::use_case_err, MappedErrors},
};
use log::error;
use uuid::Uuid;

pub(super) async fn notify_internal_user(
    user_id: Uuid,
    token_secret: TokenSecret,
    email: Email,
    frontend_url_redirect: String,
    token_registration_repo: Box<&dyn SessionTokenRegistration>,
    user_deletion_repo: Box<&dyn UserDeletion>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Issue a verification token
    // ? -----------------------------------------------------------------------

    let pasetor_token = match issue_confirmation_token_pasetor(
        user_id,
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
                user_deletion_repo.delete(user_id).await?
            {
                error!(
                    "Unable to delete user: {}. Error: {}. Generated after: {}",
                    id.to_string(),
                    msg,
                    err
                );

                return use_case_err(format!(
                    "Unexpected error on create user: {}",
                    email.get_email()
                ))
                .as_error();
            };

            return Err(err);
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify user owner
    // ? -----------------------------------------------------------------------

    let url_with_token =
        format!("{}?t={}", frontend_url_redirect, pasetor_token);

    if let Err(err) = message_sending_repo
        .send(Message {
            from: Email::from_string(token_secret.token_email_notifier)?,
            to: email,
            cc: None,
            subject: String::from("Action required: Confirm your email address"),
            message_head: Some(
                "You must confirm your email address to complete your registration process"
                .to_string()
            ),
            message_body: format!(
                "Use the follow link to validate your identity:
                <br />
                <a href={}>Activate Account</a>
                <br />
                or copy and paste the follow link on your browser:
                <br />
                {}",
                url_with_token,
                url_with_token,
            ),
            message_footer: None,
        })
        .await
    {
        return use_case_err(format!("Unable to send email: {err}")).as_error();
    };

    Ok(())
}
