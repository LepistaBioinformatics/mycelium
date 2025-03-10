use crate::{
    domain::{
        dtos::{
            email::Email, native_error_codes::NativeErrorCodes,
            token::PasswordChangeTokenMeta, user::PasswordHash,
        },
        entities::{
            LocalMessageWrite, TokenInvalidation, UserFetching, UserUpdating,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::dispatch_notification,
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

#[tracing::instrument(name = "check_token_and_reset_password", skip_all)]
pub async fn check_token_and_reset_password(
    token: String,
    email: Email,
    new_password: String,
    life_cycle_settings: AccountLifeCycle,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_updating_repo: Box<&dyn UserUpdating>,
    token_invalidation_repo: Box<&dyn TokenInvalidation>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let target_user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!("User not found: {}", email.email()))
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
                    email = email.email()
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
                email.email()
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
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    if let Err(err) = dispatch_notification(
        vec![("verification_code", meta.get_token())],
        "email/password-reset-confirmation",
        life_cycle_settings,
        email,
        None,
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
