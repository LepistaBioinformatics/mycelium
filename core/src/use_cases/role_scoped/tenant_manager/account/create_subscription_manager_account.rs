use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account,
        guest_role::{GuestRole, Permission},
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        written_by::WrittenBy,
    },
    entities::{AccountRegistration, GuestRoleRegistration},
};

use futures::future;
use mycelium_base::{
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::Instrument;
use uuid::Uuid;

/// Create a subscription manager account
///
/// The subscription manager account should be tenant-scoped and use the
/// AccountType::RoleAssociated actor associated option.
///
#[tracing::instrument(
    name = "create_subscription_manager_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, account_registration_repo, guest_role_registration_repo)
)]
pub async fn create_subscription_manager_account(
    profile: Profile,
    tenant_id: Uuid,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<CreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    tracing::trace!("Starting to create a subscription manager account");

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile
        .get_tenant_wide_permission_or_error(tenant_id, Permission::Write)?;

    // ? -----------------------------------------------------------------------
    // ? Get or create the role
    //
    // The role should be fetched from the database.
    // ? -----------------------------------------------------------------------

    let (id, slug, description, children, is_system) = (
        None,
        SystemActor::TenantManager.to_string(),
        Some(format!(
            "Subscription manager account for tenant: {}",
            tenant_id
        )),
        None,
        true,
    );

    // ? -----------------------------------------------------------------------
    // ? Get or create the read/write roles
    //
    // The roles are fetched from the database if exists, otherwise they are
    // created.
    // ? -----------------------------------------------------------------------

    let (read_role_result, write_role_result) = future::join(
        guest_role_registration_repo
            .get_or_create(GuestRole::new(
                id,
                slug.to_owned(),
                description.to_owned(),
                Permission::Read,
                children.to_owned(),
                is_system.to_owned(),
            ))
            .instrument(span.clone()),
        guest_role_registration_repo
            .get_or_create(GuestRole::new(
                id,
                slug.to_owned(),
                description,
                Permission::Write,
                children,
                is_system,
            ))
            .instrument(span.clone()),
    )
    .await;

    let read_role_id = match read_role_result? {
        GetOrCreateResponseKind::Created(role) => role,
        GetOrCreateResponseKind::NotCreated(role, _) => role,
    }
    .id
    .map_or_else(
        || {
            use_case_err(format!("Role ID is not set: {}", tenant_id))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        },
        |id| Ok(id),
    )?;

    let write_role_id = match write_role_result? {
        GetOrCreateResponseKind::Created(role) => role,
        GetOrCreateResponseKind::NotCreated(role, _) => role,
    }
    .id
    .map_or_else(
        || {
            use_case_err(format!("Role ID is not set: {}", tenant_id))
                .with_code(NativeErrorCodes::MYC00003)
                .as_error()
        },
        |id| Ok(id),
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account = Account::new_role_related_account(
        format!("tid/{}/{}", tenant_id, slug),
        tenant_id,
        read_role_id,
        write_role_id,
        slug,
        true,
        Some(WrittenBy::new_from_account(profile.acc_id)),
    );

    unchecked_account.is_checked = true;

    account_registration_repo
        .create_subscription_account(unchecked_account, tenant_id)
        .instrument(span)
        .await
}
