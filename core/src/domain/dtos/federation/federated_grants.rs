use super::IssuerNamespace;
use crate::domain::dtos::profile::LicensedResource;

use serde::{Deserialize, Serialize};

/// Grants a peer holds for a subject, with every id in the peer's namespace
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct FederatedGrants(pub Vec<LicensedResource>);

impl FederatedGrants {
    /// The tenants and accounts these grants point at, without repeats
    pub fn namespace(&self) -> IssuerNamespace {
        let mut namespace = IssuerNamespace::default();

        for grant in &self.0 {
            push_once(&mut namespace.tenants, grant.tenant_id);
            push_once(&mut namespace.accounts, grant.acc_id);
        }

        namespace
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

fn push_once(ids: &mut Vec<uuid::Uuid>, id: uuid::Uuid) {
    if !ids.contains(&id) {
        ids.push(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::guest_role::Permission;
    use uuid::Uuid;

    fn grant(tenant_id: Uuid, acc_id: Uuid) -> LicensedResource {
        LicensedResource {
            acc_id,
            sys_acc: false,
            tenant_id,
            acc_name: "account-01".to_string(),
            role: "reader".to_string(),
            role_id: Uuid::new_v4(),
            perm: Permission::Read,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }
    }

    #[test]
    fn namespace_lists_each_id_once() {
        let tenant = Uuid::new_v4();
        let (first, second) = (Uuid::new_v4(), Uuid::new_v4());
        let grants = FederatedGrants(vec![
            grant(tenant, first),
            grant(tenant, second),
            grant(tenant, first),
        ]);

        let namespace = grants.namespace();

        assert_eq!(namespace.tenants, vec![tenant]);
        assert_eq!(namespace.accounts, vec![first, second]);
        assert_eq!(namespace.ids(), vec![tenant, first, second]);
    }
}
