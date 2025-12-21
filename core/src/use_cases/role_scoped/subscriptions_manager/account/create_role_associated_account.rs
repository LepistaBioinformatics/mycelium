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
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
use tracing::Instrument;
use uuid::Uuid;

/// Create a role associated account
///
/// The role associated account should be tenant-scoped and use the
/// AccountType::RoleAssociated actor associated option.
///
#[tracing::instrument(
    name = "create_role_associated_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
        correspondence_id = tracing::field::Empty
    ),
    skip(profile, account_registration_repo, guest_role_registration_repo)
)]
pub async fn create_role_associated_account(
    profile: Profile,
    tenant_id: Uuid,
    account_name: String,
    role_name: String,
    role_description: String,
    guest_role_registration_repo: Box<&dyn GuestRoleRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let correspondence_id = Uuid::new_v4();

    let span = tracing::Span::current();
    span.record("correspondence_id", Some(correspondence_id.to_string()));

    tracing::trace!("Starting to create a role associated account");

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let is_owner = profile.with_tenant_ownership_or_error(tenant_id).is_ok();

    let has_access = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_account_or_error()
        .is_ok();

    if ![is_owner, has_access].iter().any(|&x| x) {
        return use_case_err(
            "Insufficient privileges to create a role associated account",
        )
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Get or create the role
    //
    // The role should be fetched from the database.
    // ? -----------------------------------------------------------------------

    let (id, role_slug, description, children, is_system) = (
        None,
        slugify!(role_name.as_str()),
        Some(role_description),
        None,
        false,
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
                role_slug.to_owned(),
                description.to_owned(),
                Permission::Read,
                children.to_owned(),
                is_system.to_owned(),
            ))
            .instrument(span.clone()),
        guest_role_registration_repo
            .get_or_create(GuestRole::new(
                id,
                role_slug.to_owned(),
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
        #[allow(clippy::needless_collect)]
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
        #[allow(clippy::needless_collect)]
        |id| Ok(id),
    )?;

    // ? -----------------------------------------------------------------------
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let account_name_base_slug = slugify!(format!(
        "tid/{}/role/{}/role-associated-account",
        tenant_id, role_slug
    )
    .as_str());

    let mut unchecked_account = Account::new_role_related_account(
        account_name,
        tenant_id,
        read_role_id,
        write_role_id,
        role_slug,
        false,
        Some(WrittenBy::new_from_account(profile.acc_id)),
    );

    unchecked_account.slug = account_name_base_slug;
    unchecked_account.is_checked = true;

    account_registration_repo
        .get_or_create_role_related_account(unchecked_account)
        .instrument(span)
        .await
}
