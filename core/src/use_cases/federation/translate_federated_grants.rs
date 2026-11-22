use crate::domain::dtos::{
    federation::ActiveTranslation,
    profile::{FederatedResources, LicensedResource},
};

/// The local grants the enabled rules make of the federated ones (FED-48 …
/// FED-52)
///
/// A rule applies only to the grants of its own peer; system grants are
/// never translated, and the issuer's permission only restricts. Computed
/// per request, never stored: a rule change or a tombstone acts on the next
/// request. Identical results are kept once.
///
pub fn translate_federated_grants(
    federated: &FederatedResources,
    rules: &[ActiveTranslation],
) -> Vec<LicensedResource> {
    let mut translated: Vec<LicensedResource> = vec![];

    for resource in &federated.0 {
        let local = rules
            .iter()
            .filter(|rule| rule.peer_instance_id == resource.issuer)
            .filter_map(|rule| rule.translate(&resource.grant));

        for grant in local {
            if !translated.contains(&grant) {
                translated.push(grant);
            }
        }
    }

    translated
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        federation::{IssuerGrantMatch, TranslationTarget},
        guest_role::Permission,
        profile::FederatedResource,
    };

    use uuid::Uuid;

    fn federated(issuer: Uuid, tenant_id: Uuid) -> FederatedResource {
        FederatedResource {
            issuer,
            grant: LicensedResource {
                acc_id: Uuid::new_v4(),
                sys_acc: false,
                tenant_id,
                acc_name: "account-01".to_string(),
                role: "editor".to_string(),
                role_id: Uuid::new_v4(),
                perm: Permission::Write,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            },
        }
    }

    fn rule(peer_instance_id: Uuid, tenant_id: Uuid) -> ActiveTranslation {
        ActiveTranslation {
            peer_instance_id,
            issuer: IssuerGrantMatch {
                tenant_id,
                role: "editor".to_string(),
                min_perm: Permission::Read,
            },
            target: TranslationTarget {
                account_id: Uuid::new_v4(),
                account_name: "local-account".to_string(),
                tenant_id: Uuid::new_v4(),
                role_id: Uuid::new_v4(),
                role_slug: "reviewer".to_string(),
            },
            perm: Permission::Read,
        }
    }

    #[test]
    fn a_rule_applies_to_its_own_peers_grants_once() {
        let (issuer, other, tenant) =
            (Uuid::new_v4(), Uuid::new_v4(), Uuid::new_v4());
        let grants = FederatedResources(vec![
            federated(issuer, tenant),
            federated(issuer, tenant),
            federated(other, tenant),
        ]);
        let rules = [rule(issuer, tenant)];

        let translated = translate_federated_grants(&grants, &rules);

        assert_eq!(translated.len(), 1);
        assert_eq!(translated[0].acc_id, rules[0].target.account_id);
        assert_eq!(translated[0].perm, Permission::Read);
    }

    #[test]
    fn nothing_without_a_matching_rule() {
        let issuer = Uuid::new_v4();
        let grants =
            FederatedResources(vec![federated(issuer, Uuid::new_v4())]);

        assert!(translate_federated_grants(&grants, &[]).is_empty());
        let elsewhere = [rule(issuer, Uuid::new_v4())];
        assert!(translate_federated_grants(&grants, &elsewhere).is_empty());
    }
}
