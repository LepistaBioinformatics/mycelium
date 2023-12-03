use crate::domain::{
    dtos::{
        email::Email,
        user::{Provider, User},
    },
    entities::UserFetching,
};

use clean_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

pub async fn check_email_password_validity(
    email: Email,
    password: String,
    user_fetching_repo: Box<&dyn UserFetching>,
) -> Result<(bool, Option<User>), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo.get(None, Some(email), None).await {
        Ok(FetchResponseKind::Found(user)) => user,
        _ => return Ok((false, None)),
    };

    // ? -----------------------------------------------------------------------
    // ? Check if user is active
    // ? -----------------------------------------------------------------------

    let user = match user.is_active {
        true => user,
        false => return Ok((false, None)),
    };

    // ? -----------------------------------------------------------------------
    // ? Get credentials
    // ? -----------------------------------------------------------------------

    let credentials = match user.provider() {
        None => return Ok((false, None)),
        Some(provider) => match provider {
            Provider::External(_) => return Ok((false, None)),
            Provider::Internal(credentials) => credentials,
        },
    };

    // ? -----------------------------------------------------------------------
    // ? Check password validity
    // ? -----------------------------------------------------------------------

    match credentials.check_password(password.as_bytes()) {
        Err(_) => Ok((false, None)),
        Ok(_) => Ok((true, Some(user))),
    }
}
