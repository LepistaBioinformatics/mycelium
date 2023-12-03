use super::verify_confirmation_token_pasetor::verify_confirmation_token_pasetor;
use crate::domain::{
    dtos::{session_token::TokenSecret, user::User},
    entities::{
        SessionTokenDeletion, SessionTokenFetching, UserFetching, UserUpdating,
    },
};

use clean_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{factories::use_case_err, MappedErrors},
};

pub async fn check_token_and_activate_user(
    token: String,
    is_for_password_change: Option<bool>,
    token_secret: TokenSecret,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_updating_repo: Box<&dyn UserUpdating>,
    token_fetching_repo: Box<&dyn SessionTokenFetching>,
    token_deletion_repo: Box<&dyn SessionTokenDeletion>,
) -> Result<User, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Validate token
    // ? -----------------------------------------------------------------------

    let session_token = verify_confirmation_token_pasetor(
        token,
        is_for_password_change,
        token_secret,
        token_fetching_repo,
        token_deletion_repo,
    )
    .await?;

    let id = session_token.user_id;

    // ? -----------------------------------------------------------------------
    // ? Fetch user with id contained in token
    // ? -----------------------------------------------------------------------

    let mut inactive_user =
        match user_fetching_repo.get(Some(id), None, None).await? {
            FetchResponseKind::NotFound(_) => {
                return use_case_err(format!(
                    "User with id {} not found.",
                    id.to_string()
                ))
                .as_error()
            }
            FetchResponseKind::Found(user) => user,
        };

    inactive_user.is_active = true;

    // ? -----------------------------------------------------------------------
    // ? Activate user with id contained in token
    // ? -----------------------------------------------------------------------

    match user_updating_repo.update(inactive_user).await? {
        UpdatingResponseKind::NotUpdated(_, msg) => {
            return use_case_err(format!(
                "User with id {} could not be activated: {}",
                id.to_string(),
                msg
            ))
            .as_error()
        }
        UpdatingResponseKind::Updated(user) => Ok(user),
    }
}
