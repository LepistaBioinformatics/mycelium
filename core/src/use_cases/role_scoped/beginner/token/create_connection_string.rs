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
    settings::DEFAULT_TENANT_ID_KEY,
    use_cases::support::dispatch_notification,
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
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

    // ? -----------------------------------------------------------------------
    // ? Register the token
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::NotCreated(_, msg) = token_registration_repo
        .create_connection_string(
            role_scoped_connection_string.to_owned(),
            expires_at,
        )
        .await?
    {
        tracing::error!("Unable to register connection string: {msg}");
        return use_case_err("Unable to register token").as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    let mut parameters = vec![(
        "expires_in",
        format_expiration_as_human_readable(expiration),
    )];

    if let Some(t_id) = tenant_id {
        parameters.push((DEFAULT_TENANT_ID_KEY, t_id.to_string()));
    }

    if let Err(err) = dispatch_notification(
        parameters,
        "email/create-connection-string",
        life_cycle_settings,
        Email::from_string(owner.email.to_owned())?,
        None,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
    {
        tracing::error!("Unable to send email: {err}");
        return use_case_err("Unable to notify user")
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Send user the token
    // ? -----------------------------------------------------------------------

    Ok(role_scoped_connection_string.scope.to_string())
}

fn format_expiration_as_human_readable(expiration: i64) -> String {
    let duration = Duration::seconds(expiration);
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;

    if days > 0 {
        return format!("{days}d");
    }

    if hours > 0 {
        return format!("{hours}h");
    }

    format!("{minutes}m")
}
