use crate::domain::dtos::{guest_role::Permission, profile::LicensedResource};

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// Which issuer grants a rule picks: one role on one of the issuer's
/// tenants, with at least `min_perm`
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IssuerGrantMatch {
    pub tenant_id: Uuid,
    pub role: String,
    pub min_perm: Permission,
}

impl IssuerGrantMatch {
    /// Never a system grant (FED-50, I-4)
    pub fn matches(&self, grant: &LicensedResource) -> bool {
        !grant.sys_acc
            && grant.tenant_id == self.tenant_id
            && grant.role == self.role
            && grant.perm.to_i32() >= self.min_perm.to_i32()
    }
}

/// The local grant a rule yields, on a pre-existing account and guest role
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalGrant {
    pub account_id: Uuid,
    pub guest_role_id: Uuid,
    pub perm: Permission,
}

/// An operator rule turning a peer's grant into a local one (FED-48)
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct GrantTranslationRule {
    pub id: Uuid,
    pub peer_id: Uuid,
    pub issuer: IssuerGrantMatch,
    pub local: LocalGrant,
    pub enabled: bool,
}

/// A rule to create
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NewGrantTranslationRule {
    pub peer_id: Uuid,
    pub issuer: IssuerGrantMatch,
    pub local: LocalGrant,
}

/// The local account and guest role a rule targets, as they are now
#[derive(Clone, Debug, PartialEq)]
pub struct TranslationTarget {
    pub account_id: Uuid,
    pub account_name: String,
    pub tenant_id: Uuid,
    pub role_id: Uuid,
    pub role_slug: String,
}

/// An enabled rule with its target, ready to apply at read time
#[derive(Clone, Debug, PartialEq)]
pub struct ActiveTranslation {
    pub peer_instance_id: Uuid,
    pub issuer: IssuerGrantMatch,
    pub target: TranslationTarget,
    pub perm: Permission,
}

impl ActiveTranslation {
    /// The local grant for an issuer grant the rule matches; the issuer's
    /// permission only restricts (FED-51, I-5)
    pub fn translate(
        &self,
        grant: &LicensedResource,
    ) -> Option<LicensedResource> {
        if !self.issuer.matches(grant) {
            return None;
        }

        let perm =
            Permission::from_i32(grant.perm.to_i32().min(self.perm.to_i32()));

        Some(LicensedResource {
            acc_id: self.target.account_id,
            sys_acc: false,
            tenant_id: self.target.tenant_id,
            acc_name: self.target.account_name.clone(),
            role: self.target.role_slug.clone(),
            role_id: self.target.role_id,
            perm,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn grant(
        perm: Permission,
        sys_acc: bool,
        tenant_id: Uuid,
    ) -> LicensedResource {
        LicensedResource {
            acc_id: Uuid::new_v4(),
            sys_acc,
            tenant_id,
            acc_name: "account-01".to_string(),
            role: "editor".to_string(),
            role_id: Uuid::new_v4(),
            perm,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }
    }

    fn rule(
        tenant_id: Uuid,
        min_perm: Permission,
        perm: Permission,
    ) -> ActiveTranslation {
        ActiveTranslation {
            peer_instance_id: Uuid::new_v4(),
            issuer: IssuerGrantMatch {
                tenant_id,
                role: "editor".to_string(),
                min_perm,
            },
            target: TranslationTarget {
                account_id: Uuid::new_v4(),
                account_name: "local-account".to_string(),
                tenant_id: Uuid::new_v4(),
                role_id: Uuid::new_v4(),
                role_slug: "reviewer".to_string(),
            },
            perm,
        }
    }

    #[test]
    fn a_matching_grant_becomes_the_rules_local_grant() {
        let tenant = Uuid::new_v4();
        let rule = rule(tenant, Permission::Read, Permission::Write);

        let local = rule
            .translate(&grant(Permission::Write, false, tenant))
            .unwrap();

        assert_eq!(local.acc_id, rule.target.account_id);
        assert_eq!(local.role, "reviewer");
        assert_eq!(local.perm, Permission::Write);
        assert!(!local.sys_acc);
    }

    #[test]
    fn the_issuers_permission_only_restricts() {
        let tenant = Uuid::new_v4();
        let rule = rule(tenant, Permission::Read, Permission::Write);

        let local = rule
            .translate(&grant(Permission::Read, false, tenant))
            .unwrap();

        assert_eq!(local.perm, Permission::Read);
    }

    #[test]
    fn system_grants_other_tenants_and_weaker_grants_are_never_translated() {
        let tenant = Uuid::new_v4();
        let writers_only = rule(tenant, Permission::Write, Permission::Write);

        assert!(writers_only
            .translate(&grant(Permission::Write, true, tenant))
            .is_none());
        assert!(writers_only
            .translate(&grant(Permission::Write, false, Uuid::new_v4()))
            .is_none());
        assert!(writers_only
            .translate(&grant(Permission::Read, false, tenant))
            .is_none());
    }
}
