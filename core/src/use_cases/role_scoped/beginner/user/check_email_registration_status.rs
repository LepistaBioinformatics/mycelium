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
    pub account_created: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum EmailRegistrationStatus {
    NotRegistered(String),
    WaitingActivation(String),
    RegisteredWithInternalProvider(RegisteredWithProvider),
    RegisteredWithExternalProvider(RegisteredWithProvider),
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
#[tracing::instrument(name = "check_email_registration_status", skip_all)]
pub async fn check_email_registration_status(
    email: Email,
    user_fetching_repo: Box<&dyn UserFetching>,
) -> Result<EmailRegistrationStatus, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch user from data storage
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo
        .get_user_by_email(email.to_owned())
        .await?
    {
        FetchResponseKind::Found(user) => user,
        FetchResponseKind::NotFound(_) => {
            return Ok(NotRegistered(email.email()))
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check for user activation
    // ? -----------------------------------------------------------------------

    if !user.is_active {
        return Ok(WaitingActivation(email.email()));
    }

    // ? -----------------------------------------------------------------------
    // ? Initialize the response user
    // ? -----------------------------------------------------------------------

    let registered_user = RegisteredWithProvider {
        email: email.to_owned(),
        provider: user.provider(),
        account_created: user.account.is_some(),
    };

    match user.with_internal_provider()? {
        true => Ok(RegisteredWithInternalProvider(registered_user)),
        false => Ok(RegisteredWithExternalProvider(registered_user)),
    }
}
