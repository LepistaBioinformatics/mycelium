use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            security_group::PermissionedRole,
            token::{UserAccountConnectionString, UserAccountScope},
        },
        entities::{LocalMessageWrite, TenantFetching, TokenRegistration},
    },
    models::AccountLifeCycle,
};

use super::register_and_notify_connection_string::register_and_notify_connection_string;

use chrono::{Duration, Local};
use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

/// Create a connection string
///
/// This function creates a connection string that is associated with the user
/// account. The connection string has the same permissions of the user account,
/// but the tenant_id and permissioned_roles can be specified to create a
/// connection string that is scoped to a specific tenant and/or roles.
///
#[tracing::instrument(
    name = "create_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn create_connection_string(
    profile: Profile,
    name: String,
    expiration: i64,
    tenant_id: Option<Uuid>,
    subscription_account_id: Option<Uuid>,
    roles: Option<Vec<PermissionedRole>>,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Build the scoped account token
    // ? -----------------------------------------------------------------------

    let mut owners = profile.owners;
    owners.sort_by_key(|owner| owner.email.to_owned());

    let owner = match owners.iter().find(|owner| owner.is_principal) {
        Some(owner) => owner,
        None => return use_case_err(
            "Action restricted to the principal user from the current profile",
        )
        .with_code(NativeErrorCodes::MYC00013)
        .as_error(),
    };

    let expires_at = Local::now() + Duration::seconds(expiration);

    let mut role_scope = UserAccountScope::new(
        profile.acc_id,
        expires_at,
        roles,
        tenant_id,
        subscription_account_id,
        life_cycle_settings.to_owned(),
    )
    .await?;

    let role_scoped_connection_string =
        UserAccountConnectionString::new_signed_token(
            &mut role_scope,
            profile.acc_id,
            Email::from_string(owner.email.to_owned())?,
            life_cycle_settings.to_owned(),
            Some(name),
        )
        .await?;

    register_and_notify_connection_string(
        role_scoped_connection_string,
        expires_at,
        expiration,
        tenant_id,
        Email::from_string(owner.email.to_owned())?,
        life_cycle_settings,
        token_registration_repo,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
}
