use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            route_type::PermissionedRoles,
            token::{RoleScopedConnectionString, RoleWithPermissionsScope},
        },
        entities::{MessageSending, TokenRegistration},
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
    name = "create_role_associated_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn create_role_associated_connection_string(
    profile: Profile,
    tenant_id: Uuid,
    role_id: Uuid,
    permissioned_roles: PermissionedRoles,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![ActorName::GuestManager])?;

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

    let expires_at =
        Local::now() + Duration::seconds(life_cycle_settings.token_expiration);

    let mut role_scope = RoleWithPermissionsScope::new(
        tenant_id,
        role_id,
        permissioned_roles.to_owned(),
        expires_at,
        life_cycle_settings.to_owned(),
    )?;

    let role_scoped_connection_string =
        RoleScopedConnectionString::new_signed_token(
            &mut role_scope,
            owner.id,
            Email::from_string(owner.email.to_owned())?,
            life_cycle_settings.to_owned(),
        )?;

    // ? -----------------------------------------------------------------------
    // ? Register the token
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::NotCreated(_, msg) = token_registration_repo
        .create_role_scoped_connection_string(
            role_scoped_connection_string.to_owned(),
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
        ("target_id", role_id.to_string().to_uppercase()),
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
        "email/create-connection-string.jinja",
        life_cycle_settings,
        Email::from_string(owner.email.to_owned())?,
        None,
        String::from("[Connection String] New connection string created"),
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

    Ok(role_scoped_connection_string.scope.to_string())
}
