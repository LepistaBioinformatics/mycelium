use crate::domain::{dtos::email::Email, entities::UserFetching};
use clean_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum EmailRegistrationStatus {
    RegisteredAndInternal(Email),
    RegisteredButExternal(Email),
    NotRegistered(Option<String>),
}

/// Check if the user was already registered in Mycelium or not.
///
/// Case 1: The user was registered in Mycelium with an external provider.
///
/// Case 2: The user was registered in Mycelium with an internal provider.
///
/// Case 3: The user was not registered in Mycelium.
///
/// ----------------------------------------------------------------------------
///
/// This function should be used during the double step of the user
/// authentication. The first step is to check if the user is registered in
/// Mycelium.
///
pub async fn check_email_registration_status(
    email: Email,
    user_fetching_repo: Box<&dyn UserFetching>,
) -> Result<EmailRegistrationStatus, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from data storage
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo
        .get(None, Some(email.to_owned()), None)
        .await?
    {
        FetchResponseKind::Found(user) => user,
        FetchResponseKind::NotFound(email) => {
            return Ok(EmailRegistrationStatus::NotRegistered(email))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if user is internal
    // ? -----------------------------------------------------------------------

    match user.is_internal_or_error() {
        Err(err) => return Err(err),
        Ok(res) => match res {
            true => Ok(EmailRegistrationStatus::RegisteredAndInternal(email)),
            false => Ok(EmailRegistrationStatus::RegisteredButExternal(email)),
        },
    }
}
