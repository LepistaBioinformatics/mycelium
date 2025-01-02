use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            route_type::PermissionedRoles,
            token::{
                AccountScopedConnectionString,
                AccountWithPermissionedRolesScope,
            },
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
    name = "create_default_account_associated_connection_string",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn create_default_account_associated_connection_string(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    expiration: i64,
    permissioned_roles: PermissionedRoles,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile.get_default_write_ids_or_error(vec![SystemActor::GuestManager])?;

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

    let mut account_scope = AccountWithPermissionedRolesScope::new(
        tenant_id,
        account_id,
        permissioned_roles.to_owned(),
        expires_at,
        life_cycle_settings.to_owned(),
    )
    .await?;

    let account_scoped_connection_string =
        AccountScopedConnectionString::new_signed_token(
            &mut account_scope,
            owner.id,
            Email::from_string(owner.email.to_owned())?,
            life_cycle_settings.to_owned(),
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Register the token
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::NotCreated(_, msg) = token_registration_repo
        .create_account_scoped_connection_string(
            account_scoped_connection_string.to_owned(),
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
        ("target_id", account_id.to_string().to_uppercase()),
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
        String::from("New connection string created"),
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

    Ok(account_scoped_connection_string.scope.to_string())
}
