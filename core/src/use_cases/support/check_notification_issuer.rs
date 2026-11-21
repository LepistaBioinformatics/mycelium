use crate::domain::dtos::{
    native_error_codes::NativeErrorCodes, profile::Profile,
    service_notification_config::ServiceNotificationConfig,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

/// Check if a profile may issue or use notification strings of a service
///
/// The owner and the account must be active, approved and not archived. The
/// profile must hold at least one of the service `issuerRoles` (on the tenant,
/// when one is informed), and the tenant must be listed in `issuerTenants`
/// when that list is not empty. Every failure returns MYC00037.
///
/// Called at issuance with the request profile and at send time with a fresh
/// one.
#[tracing::instrument(name = "check_notification_issuer", skip_all)]
pub fn check_notification_issuer(
    profile: &Profile,
    tenant_id: Option<Uuid>,
    config: &ServiceNotificationConfig,
) -> Result<(), MappedErrors> {
    if !issuer_account_is_in_good_standing(profile) {
        return issuer_not_allowed("Issuer account or owner is not active");
    }

    if !issuer_tenant_is_allowed(tenant_id, &config.issuer_tenants) {
        return issuer_not_allowed("Tenant not allowed to issue notifications");
    }

    let scoped_profile = match tenant_id {
        Some(tenant_id) => profile.on_tenant(tenant_id),
        None => profile.to_owned(),
    };

    if scoped_profile
        .with_roles(config.issuer_roles.to_owned())
        .get_ids_or_error()
        .is_err()
    {
        return issuer_not_allowed("Issuer lacks the service issuer roles");
    }

    Ok(())
}

fn issuer_account_is_in_good_standing(profile: &Profile) -> bool {
    profile.owner_is_active
        && profile.account_is_active
        && profile.account_was_approved
        && !profile.account_was_archived
}

fn issuer_tenant_is_allowed(
    tenant_id: Option<Uuid>,
    issuer_tenants: &[Uuid],
) -> bool {
    if issuer_tenants.is_empty() {
        return true;
    }

    let Some(tenant_id) = tenant_id else {
        return false;
    };

    issuer_tenants.contains(&tenant_id)
}

fn issuer_not_allowed(message: &str) -> Result<(), MappedErrors> {
    use_case_err(message)
        .with_code(NativeErrorCodes::MYC00037)
        .with_exp_true()
        .as_error()
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        guest_role::Permission,
        profile::{LicensedResource, LicensedResources},
    };

    const ISSUER_ROLE: &str = "service-01-admin";

    fn config(issuer_tenants: Vec<Uuid>) -> ServiceNotificationConfig {
        serde_json::from_value(serde_json::json!({
            "senderTag": "service-01",
            "displayName": "Service 01",
            "issuerRoles": [ISSUER_ROLE],
            "issuerTenants": issuer_tenants,
            "recipientPolicy": "owner",
            "templates": ["welcome"],
        }))
        .unwrap()
    }

    fn profile_with_role(role: &str, tenant_id: Uuid) -> Profile {
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
                acc_name: "Account".to_string(),
                sys_acc: false,
                role: role.to_string(),
                perm: Permission::Write,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            }])),
            None,
        )
    }

    fn assert_not_allowed(result: Result<(), MappedErrors>) {
        let error = result.unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00037]), "{error}");
    }

    #[test]
    fn issuer_with_the_role_on_the_tenant_is_allowed() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_with_role(ISSUER_ROLE, tenant_id);

        check_notification_issuer(&profile, Some(tenant_id), &config(vec![]))
            .unwrap();
    }

    #[test]
    fn issuer_without_the_role_is_rejected() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_with_role("other-role", tenant_id);

        assert_not_allowed(check_notification_issuer(
            &profile,
            Some(tenant_id),
            &config(vec![]),
        ));
    }

    #[test]
    fn issuer_with_the_role_on_another_tenant_is_rejected() {
        let profile = profile_with_role(ISSUER_ROLE, Uuid::new_v4());

        assert_not_allowed(check_notification_issuer(
            &profile,
            Some(Uuid::new_v4()),
            &config(vec![]),
        ));
    }

    #[test]
    fn tenant_listed_in_issuer_tenants_is_allowed() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_with_role(ISSUER_ROLE, tenant_id);

        check_notification_issuer(
            &profile,
            Some(tenant_id),
            &config(vec![tenant_id]),
        )
        .unwrap();
    }

    #[test]
    fn tenant_missing_from_issuer_tenants_is_rejected() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_with_role(ISSUER_ROLE, tenant_id);

        assert_not_allowed(check_notification_issuer(
            &profile,
            Some(tenant_id),
            &config(vec![Uuid::new_v4()]),
        ));
    }

    #[test]
    fn no_tenant_checks_the_role_on_any_tenant() {
        let profile = profile_with_role(ISSUER_ROLE, Uuid::new_v4());

        check_notification_issuer(&profile, None, &config(vec![])).unwrap();
    }

    #[test]
    fn no_tenant_is_rejected_when_issuer_tenants_is_set() {
        let tenant_id = Uuid::new_v4();
        let profile = profile_with_role(ISSUER_ROLE, tenant_id);

        assert_not_allowed(check_notification_issuer(
            &profile,
            None,
            &config(vec![tenant_id]),
        ));
    }

    #[test]
    fn each_failing_status_flag_rejects_the_issuer() {
        let tenant_id = Uuid::new_v4();
        let healthy = profile_with_role(ISSUER_ROLE, tenant_id);

        let mut inactive_owner = healthy.clone();
        inactive_owner.owner_is_active = false;

        let mut inactive_account = healthy.clone();
        inactive_account.account_is_active = false;

        let mut unapproved_account = healthy.clone();
        unapproved_account.account_was_approved = false;

        let mut archived_account = healthy;
        archived_account.account_was_archived = true;

        for profile in [
            inactive_owner,
            inactive_account,
            unapproved_account,
            archived_account,
        ] {
            assert_not_allowed(check_notification_issuer(
                &profile,
                Some(tenant_id),
                &config(vec![]),
            ));
        }
    }
}
