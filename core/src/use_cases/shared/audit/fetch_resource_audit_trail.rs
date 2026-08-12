// ? ---------------------------------------------------------------------------
// ? fetch_resource_audit_trail
//
// Shared read-side use case for `resource_audit_log`. Lives under
// `shared/audit`, not under a single role scope, because the same function
// serves staff, tenant owners/managers, and personal account owners -- the
// permission branching happens inside it instead of via separate role-scoped
// copies. See `design.md`'s "fetch_resource_audit_trail (use case, read
// side)" section for the confirmed permission rule.
// ? ---------------------------------------------------------------------------

use crate::domain::{
    dtos::{
        guest_role::Permission,
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        resource_audit_log::{ResourceAuditLog, ResourceAuditResourceType},
    },
    entities::ResourceAuditLogFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(name = "fetch_resource_audit_trail", skip_all)]
pub async fn fetch_resource_audit_trail(
    profile: Profile,
    resource_type: ResourceAuditResourceType,
    resource_id: Uuid,
    tenant_id: Option<Uuid>,
    resource_owner_account_id: Option<Uuid>,
    fetching_repo: Box<&dyn ResourceAuditLogFetching>,
) -> Result<FetchManyResponseKind<ResourceAuditLog>, MappedErrors> {
    if profile.is_staff {
        return fetching_repo
            .list_by_resource(resource_type, resource_id)
            .await;
    }

    if let Some(tenant_id) = tenant_id {
        check_tenant_owner_or_manager(&profile, tenant_id)?;

        return fetching_repo
            .list_by_resource(resource_type, resource_id)
            .await;
    }

    check_personal_resource_owner(&profile, resource_owner_account_id)?;

    fetching_repo
        .list_by_resource(resource_type, resource_id)
        .await
}

/// Allow only profiles with ownership or manager standing over `tenant_id`.
///
/// Both checks are read directly off the profile's own data instead of going
/// through `Profile::get_ids_or_error` / `Profile::get_related_account_or_error`,
/// which short-circuit to `Ok` whenever the profile's *global*
/// `is_staff || is_manager` flag is set -- regardless of whether the profile
/// has any relationship to `tenant_id` specifically. Using those methods here
/// would let any globally-flagged manager read the audit trail of a tenant
/// they have no standing in.
///
/// - Ownership uses `Profile::with_tenant_ownership_or_error`, which only
///   inspects `profile.tenants_ownership` for a matching `tenant_id` and has
///   no admin/manager bypass.
/// - Manager standing filters `profile.licensed_resources` down to
///   `tenant_id` and the `TenantManager` role via `Profile::on_tenant_as_manager`
///   (a pure filter, no bypass), then checks the filtered
///   `licensed_resources` field directly for a match -- without calling the
///   terminal `get_ids_or_error()`, which is what carries the global bypass.
fn check_tenant_owner_or_manager(
    profile: &Profile,
    tenant_id: Uuid,
) -> Result<(), MappedErrors> {
    let is_tenant_owner =
        profile.with_tenant_ownership_or_error(tenant_id).is_ok();

    let is_tenant_manager = profile
        .on_tenant_as_manager(tenant_id, Permission::Read)
        .licensed_resources
        .is_some();

    if is_tenant_owner || is_tenant_manager {
        return Ok(());
    }

    use_case_err(
        "Insufficient privileges to read this audit trail: not an owner or \
         manager of the resource's tenant"
            .to_string(),
    )
    .with_code(NativeErrorCodes::MYC00019)
    .with_exp_true()
    .as_error()
}

/// Allow only the profile that owns `resource_owner_account_id`.
///
/// Compares `profile.acc_id` directly against the resource's owning account,
/// mirroring how `role_scoped/beginner/account` use cases (e.g.
/// `get_my_account_details.rs`, `update_own_account_name.rs`) already
/// establish "this is my own account" -- a plain identity check, with no
/// fluent chain. `Profile::get_related_account_or_error` is deliberately
/// avoided here: it answers the broader "which accounts can I operate on"
/// question and short-circuits to `Ok` whenever the profile's global
/// `is_staff` or `is_manager` flag is set, which would wrongly grant access
/// to any globally-flagged manager regardless of account ownership.
fn check_personal_resource_owner(
    profile: &Profile,
    resource_owner_account_id: Option<Uuid>,
) -> Result<(), MappedErrors> {
    let Some(account_id) = resource_owner_account_id else {
        return use_case_err(
            "Insufficient privileges to read this audit trail: resource has \
             no owning account"
                .to_string(),
        )
        .with_code(NativeErrorCodes::MYC00019)
        .with_exp_true()
        .as_error();
    };

    if profile.acc_id == account_id {
        return Ok(());
    }

    use_case_err(
        "Insufficient privileges to read this audit trail: not the \
         resource's owning account"
            .to_string(),
    )
    .with_code(NativeErrorCodes::MYC00019)
    .with_exp_true()
    .as_error()
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        actors::SystemActor,
        dtos::{
            guest_role::Permission,
            profile::{
                LicensedResource, LicensedResources, Profile, TenantOwnership,
                TenantsOwnership,
            },
        },
        entities::MockResourceAuditLogFetching,
    };

    use chrono::Local;

    fn base_profile() -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn staff_profile() -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            true,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn profile_with_tenant_ownership(tenant_id: Uuid) -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            Some(TenantsOwnership::Records(vec![TenantOwnership {
                id: tenant_id,
                name: "Tenant Name".to_string(),
                since: Local::now(),
            }])),
        )
    }

    fn profile_with_tenant_manager_license(tenant_id: Uuid) -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            Some(LicensedResources::Records(vec![LicensedResource {
                acc_id: Uuid::new_v4(),
                tenant_id,
                role_id: Uuid::new_v4(),
                acc_name: "Tenant Manager Account".to_string(),
                sys_acc: false,
                role: SystemActor::TenantManager.to_string(),
                perm: Permission::Read,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            }])),
            None,
        )
    }

    /// A profile whose own account (`acc_id`) is `account_id`.
    ///
    /// The personal-account branch now checks direct identity
    /// (`profile.acc_id == resource_owner_account_id`), so "owning" an
    /// account means the profile's own account id matches -- not merely
    /// holding a licensed resource that references it.
    fn profile_owning_account(account_id: Uuid) -> Profile {
        Profile::new(
            vec![],
            account_id,
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn manager_profile_without_relationships() -> Profile {
        Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            true,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        )
    }

    fn mock_allowing_fetch() -> MockResourceAuditLogFetching {
        let mut mock = MockResourceAuditLogFetching::new();

        mock.expect_list_by_resource()
            .times(1)
            .returning(|_, _| Ok(FetchManyResponseKind::Found(vec![])));

        mock
    }

    fn mock_never_called() -> MockResourceAuditLogFetching {
        let mut mock = MockResourceAuditLogFetching::new();

        mock.expect_list_by_resource().times(0);

        mock
    }

    #[tokio::test]
    async fn staff_profile_is_always_allowed() {
        let mock = mock_allowing_fetch();

        let result = fetch_resource_audit_trail(
            staff_profile(),
            ResourceAuditResourceType::Account,
            Uuid::new_v4(),
            Some(Uuid::new_v4()),
            None,
            Box::new(&mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn tenant_owner_of_matching_tenant_is_allowed() {
        let tenant_id = Uuid::new_v4();
        let mock = mock_allowing_fetch();

        let result = fetch_resource_audit_trail(
            profile_with_tenant_ownership(tenant_id),
            ResourceAuditResourceType::Tenant,
            Uuid::new_v4(),
            Some(tenant_id),
            None,
            Box::new(&mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn tenant_manager_of_matching_tenant_is_allowed() {
        let tenant_id = Uuid::new_v4();
        let mock = mock_allowing_fetch();

        let result = fetch_resource_audit_trail(
            profile_with_tenant_manager_license(tenant_id),
            ResourceAuditResourceType::Tenant,
            Uuid::new_v4(),
            Some(tenant_id),
            None,
            Box::new(&mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn tenant_owner_or_manager_of_a_different_tenant_is_denied() {
        let own_tenant_id = Uuid::new_v4();
        let other_tenant_id = Uuid::new_v4();
        let mock = mock_never_called();

        let owner_result = fetch_resource_audit_trail(
            profile_with_tenant_ownership(own_tenant_id),
            ResourceAuditResourceType::Tenant,
            Uuid::new_v4(),
            Some(other_tenant_id),
            None,
            Box::new(&mock),
        )
        .await;

        assert!(owner_result.is_err());

        let manager_result = fetch_resource_audit_trail(
            profile_with_tenant_manager_license(own_tenant_id),
            ResourceAuditResourceType::Tenant,
            Uuid::new_v4(),
            Some(other_tenant_id),
            None,
            Box::new(&mock),
        )
        .await;

        assert!(manager_result.is_err());
    }

    #[tokio::test]
    async fn account_owner_of_matching_account_is_allowed() {
        let account_id = Uuid::new_v4();
        let mock = mock_allowing_fetch();

        let result = fetch_resource_audit_trail(
            profile_owning_account(account_id),
            ResourceAuditResourceType::User,
            Uuid::new_v4(),
            None,
            Some(account_id),
            Box::new(&mock),
        )
        .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn unrelated_account_owner_is_denied() {
        let account_id = Uuid::new_v4();
        let other_account_id = Uuid::new_v4();
        let mock = mock_never_called();

        let result = fetch_resource_audit_trail(
            profile_owning_account(other_account_id),
            ResourceAuditResourceType::User,
            Uuid::new_v4(),
            None,
            Some(account_id),
            Box::new(&mock),
        )
        .await;

        assert!(result.is_err());
    }

    /// Regression test: a profile with the *global* `is_manager` flag set,
    /// but with no tenant-ownership entry or manager-role licensed resource
    /// for the specific tenant being queried, must be denied. Before the
    /// fix, `Profile::get_ids_or_error` would short-circuit to `Ok` purely
    /// because `is_manager` was `true`, regardless of the tenant match.
    #[tokio::test]
    async fn manager_flag_without_matching_tenant_relationship_is_denied() {
        let tenant_id = Uuid::new_v4();
        let mock = mock_never_called();

        let result = fetch_resource_audit_trail(
            manager_profile_without_relationships(),
            ResourceAuditResourceType::Tenant,
            Uuid::new_v4(),
            Some(tenant_id),
            None,
            Box::new(&mock),
        )
        .await;

        assert!(result.is_err());
    }

    /// Regression test: a profile with the *global* `is_manager` flag set,
    /// but whose own account does not match `resource_owner_account_id`,
    /// must be denied. Before the fix,
    /// `Profile::get_related_account_or_error` would short-circuit to
    /// `Ok(RelatedAccounts::HasManagerPrivileges)` purely because
    /// `is_manager` was `true`, regardless of account ownership.
    #[tokio::test]
    async fn manager_flag_without_matching_account_relationship_is_denied() {
        let account_id = Uuid::new_v4();
        let mock = mock_never_called();

        let result = fetch_resource_audit_trail(
            manager_profile_without_relationships(),
            ResourceAuditResourceType::User,
            Uuid::new_v4(),
            None,
            Some(account_id),
            Box::new(&mock),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn profile_with_no_qualifying_standing_is_denied() {
        let mock = mock_never_called();

        let result = fetch_resource_audit_trail(
            base_profile(),
            ResourceAuditResourceType::Webhook,
            Uuid::new_v4(),
            None,
            None,
            Box::new(&mock),
        )
        .await;

        assert!(result.is_err());
    }
}
