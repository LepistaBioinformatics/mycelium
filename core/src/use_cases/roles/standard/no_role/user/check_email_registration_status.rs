use self::EmailRegistrationStatus::*;
use crate::domain::{
    dtos::{email::Email, user::Provider},
    entities::UserFetching,
};
use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct RegisteredWithProvider {
    pub email: Email,
    pub provider: Option<Provider>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum EmailRegistrationStatus {
    RegisteredAndInternal(RegisteredWithProvider),
    RegisteredButExternal(RegisteredWithProvider),
    WaitingActivation(String),
    NotRegistered(String),
}

/// Check if the user was already registered in Mycelium or not.
///
/// Case 1: The user was registered in Mycelium with an external provider.
///
/// Case 2: The user was registered in Mycelium with an internal provider.
///
/// Case 3: The user was registered in Mycelium and is waiting for activation.
///
/// Case 4: The user was not registered in Mycelium.
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
        FetchResponseKind::NotFound(_) => {
            return Ok(NotRegistered(email.get_email()))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if user is internal
    // ? -----------------------------------------------------------------------

    let registered_user = RegisteredWithProvider {
        email: email.to_owned(),
        provider: user.provider(),
    };

    match user.has_provider_or_error() {
        Err(err) => return Err(err),
        Ok(res) => match res {
            false => Ok(RegisteredButExternal(registered_user)),
            true => {
                if !user.is_active {
                    return Ok(WaitingActivation(email.get_email()));
                }

                Ok(RegisteredAndInternal(registered_user))
            }
        },
    }
}
