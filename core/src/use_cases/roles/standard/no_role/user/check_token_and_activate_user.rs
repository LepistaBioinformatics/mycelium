use crate::domain::{
    dtos::{email::Email, token::EmailConfirmationTokenMeta, user::User},
    entities::{TokenInvalidation, UserFetching, UserUpdating},
};

use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};

pub async fn check_token_and_activate_user(
    token: String,
    email: Email,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_updating_repo: Box<&dyn UserUpdating>,
    token_invalidation_repo: Box<&dyn TokenInvalidation>,
) -> Result<User, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from email
    // ? -----------------------------------------------------------------------

    let mut inactive_user = match user_fetching_repo
        .get(None, Some(email.to_owned()), None)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "User not found: {}",
                email.get_email()
            ))
            .as_error()
        }
        FetchResponseKind::Found(user) => user,
    };

    // ? -----------------------------------------------------------------------
    // ? Validate token
    // ? -----------------------------------------------------------------------

    let meta = EmailConfirmationTokenMeta::new(
        match inactive_user.id {
            Some(id) => id,
            None => {
                return use_case_err(format!(
                    "User with email {} is invalid",
                    email.get_email()
                ))
                .as_error()
            }
        },
        email.to_owned(),
        token,
    );

    let user_id = match token_invalidation_repo
        .get_and_invalidate_email_confirmation_token(meta)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Token not found or expired for user with email {}",
                email.get_email()
            ))
            .as_error()
        }
        FetchResponseKind::Found(id) => id,
    };

    // ? -----------------------------------------------------------------------
    // ? Activate user and return
    // ? -----------------------------------------------------------------------

    inactive_user.is_active = true;

    match user_updating_repo.update(inactive_user).await? {
        UpdatingResponseKind::NotUpdated(_, msg) => {
            return use_case_err(format!(
                "User with id {} could not be activated: {}",
                user_id.to_string(),
                msg
            ))
            .as_error()
        }
        UpdatingResponseKind::Updated(user) => Ok(user),
    }
}
