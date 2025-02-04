use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            route_type::PermissionedRoles,
            token::{TenantScopedConnectionString, TenantWithPermissionsScope},
        },
        entities::{LocalMessageSending, TokenRegistration},
    },
    models::AccountLifeCycle,
    use_cases::support::send_email_notification,
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "create_tenant_associated_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn create_tenant_associated_connection_string(
    profile: Profile,
    tenant_id: Uuid,
    expiration: i64,
    permissioned_roles: PermissionedRoles,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageSending>,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_ids_or_error()?;

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

    let mut tenant_scope = TenantWithPermissionsScope::new(
        tenant_id,
        permissioned_roles.to_owned(),
        expires_at,
        life_cycle_settings.to_owned(),
    )
    .await?;

    let tenant_scoped_connection_string =
        TenantScopedConnectionString::new_signed_token(
            &mut tenant_scope,
            owner.id,
            Email::from_string(owner.email.to_owned())?,
            life_cycle_settings.to_owned(),
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Register the token
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::NotCreated(_, msg) = token_registration_repo
        .create_tenant_scoped_connection_string(
            tenant_scoped_connection_string.to_owned(),
            expires_at,
        )
        .await?
    {
        return use_case_err(msg).as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    let parameters = vec![
        ("tenant_id", tenant_id.to_string().to_uppercase()),
        ("target_id", tenant_id.to_string().to_uppercase()),
        (
            "permissioned_roles",
            permissioned_roles.iter().fold(
                String::new(),
                |acc, (role, permission)| {
                    acc + &format!("{}: {}\n", role, permission.to_string())
                },
            ),
        ),
    ];

    if let Err(err) = send_email_notification(
        parameters,
        "email/create-connection-string",
        life_cycle_settings,
        Email::from_string(owner.email.to_owned())?,
        None,
        message_sending_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Send user the token
    // ? -----------------------------------------------------------------------

    Ok(tenant_scoped_connection_string.scope.to_string())
}
