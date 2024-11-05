use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            profile::Profile,
            route_type::PermissionedRoles,
            token::{AccountScope, AccountScopedConnectionStringMeta},
        },
        entities::TokenRegistration,
    },
    models::AccountLifeCycle,
};

use chrono::{Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "create_default_account_associated_token",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn create_default_account_associated_token(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    permissioned_roles: PermissionedRoles,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
) -> Result<(), MappedErrors> {
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

    let mut account_scope = AccountScope::new(
        tenant_id,
        account_id,
        permissioned_roles,
        expires_at,
        life_cycle_settings.to_owned(),
    )?;

    let account_scoped_connection_string =
        AccountScopedConnectionStringMeta::new_signed_token(
            &mut account_scope,
            owner.id,
            Email::from_string(owner.email.to_owned())?,
            life_cycle_settings,
        )?;

    // ? -----------------------------------------------------------------------
    // ? Register the token
    // ? -----------------------------------------------------------------------

    let token = match token_registration_repo
        .create_account_scoped_connection_string(
            account_scoped_connection_string.to_owned(),
            expires_at,
        )
        .await?
    {
        CreateResponseKind::Created(token) => token,
        CreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(msg).as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    unimplemented!()
}
