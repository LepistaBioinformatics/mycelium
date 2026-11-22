use super::{LicensedResource, LicensedResources, Profile};

use uuid::Uuid;

/// Opt-in to the grants peers asserted (FED-31)
///
/// Every other filter works on `licensedResources` only, so without these a
/// chain sees local grants alone, exactly as before federation.
///
/// Both must open a chain. Called after another filter they would bring back
/// federated grants that filter never saw, so they fail closed instead: the
/// result holds no licensed resource.
///
impl Profile {
    /// Add every federated grant to the local ones
    #[tracing::instrument(name = "with_federated", skip_all)]
    pub fn with_federated(&self) -> Self {
        if !self.opens_chain() {
            return self.closed("withFederated", "true".to_string());
        }

        let federated = self
            .federated_resources
            .as_ref()
            .map(|resources| resources.grants())
            .unwrap_or_default();

        let mut grants = self
            .licensed_resources
            .as_ref()
            .map(|resources| resources.to_licenses_vector())
            .unwrap_or_default();
        grants.extend(federated);

        Self {
            licensed_resources: records_or_none(grants),
            ..self
                .update_state("withFederated".to_string(), "true".to_string())
                .clone()
        }
    }

    /// Keep only the grants one issuer asserted, without the local ones, for
    /// chains with no tenant filter (role-only or system checks)
    #[tracing::instrument(name = "from_issuer", skip_all)]
    pub fn from_issuer(&self, issuer: Uuid) -> Self {
        if !self.opens_chain() {
            return self.closed("issuer", issuer.to_string());
        }

        let grants = self
            .federated_resources
            .as_ref()
            .map(|resources| resources.grants_from(issuer))
            .unwrap_or_default();

        Self {
            licensed_resources: records_or_none(grants),
            ..self
                .update_state("issuer".to_string(), issuer.to_string())
                .clone()
        }
    }
}

impl Profile {
    /// No filter ran before
    fn opens_chain(&self) -> bool {
        self.filtering_state
            .as_ref()
            .is_none_or(|state| state.is_empty())
    }

    /// The state entry recorded, and nothing left to authorize
    fn closed(&self, key: &str, value: String) -> Self {
        Self {
            licensed_resources: None,
            ..self.update_state(key.to_string(), value).clone()
        }
    }
}

fn records_or_none(grants: Vec<LicensedResource>) -> Option<LicensedResources> {
    match grants.is_empty() {
        true => None,
        false => Some(LicensedResources::Records(grants)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        guest_role::Permission,
        profile::{FederatedResource, FederatedResources},
    };

    fn grant(role: &str) -> LicensedResource {
        LicensedResource {
            acc_id: Uuid::new_v4(),
            sys_acc: false,
            tenant_id: Uuid::new_v4(),
            acc_name: "account-01".to_string(),
            role: role.to_string(),
            role_id: Uuid::new_v4(),
            perm: Permission::Read,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }
    }

    fn profile(issuers: [Uuid; 2]) -> Profile {
        let mut profile = Profile::default();
        profile.licensed_resources =
            Some(LicensedResources::Records(vec![grant("reader")]));
        profile.federated_resources = Some(FederatedResources(
            issuers
                .iter()
                .map(|issuer| FederatedResource {
                    issuer: *issuer,
                    grant: grant("reader"),
                })
                .collect(),
        ));
        profile
    }

    fn count(profile: &Profile) -> usize {
        profile
            .licensed_resources
            .as_ref()
            .map(|resources| resources.to_licenses_vector().len())
            .unwrap_or_default()
    }

    #[test]
    fn the_default_chain_sees_local_grants_only() {
        let profile = profile([Uuid::new_v4(), Uuid::new_v4()]);

        assert_eq!(count(&profile.with_roles(vec!["reader".to_string()])), 1);
    }

    #[test]
    fn with_federated_adds_every_issuers_grants() {
        let profile = profile([Uuid::new_v4(), Uuid::new_v4()]);

        assert_eq!(count(&profile.with_federated()), 3);
    }

    #[test]
    fn from_issuer_keeps_that_issuers_grants_only() {
        let issuer = Uuid::new_v4();
        let profile = profile([issuer, Uuid::new_v4()]);

        let narrowed = profile.from_issuer(issuer);

        assert_eq!(count(&narrowed), 1);
        assert_eq!(
            narrowed.licensed_resources.unwrap().to_licenses_vector()[0],
            profile.federated_resources.unwrap().0[0].grant
        );
        assert_eq!(count(&profile_without_federation().from_issuer(issuer)), 0);
    }

    fn profile_without_federation() -> Profile {
        Profile::default()
    }

    #[test]
    fn federated_grants_round_trip_with_their_issuer() {
        let profile = profile([Uuid::new_v4(), Uuid::new_v4()]);
        let json = serde_json::to_value(&profile).unwrap();

        assert!(json["federatedResources"][0]["issuer"].is_string());
        assert!(json["federatedResources"][0]["accId"].is_string());
        assert_eq!(serde_json::from_value::<Profile>(json).unwrap(), profile);
        let local_only = serde_json::to_value(Profile::default()).unwrap();
        assert!(local_only.get("federatedResources").is_none());
    }

    #[test]
    fn opt_ins_after_another_filter_fail_closed() {
        let issuer = Uuid::new_v4();
        let profile = profile([issuer, Uuid::new_v4()]);
        let federated_tenant = profile.federated_resources.clone().unwrap().0
            [0]
        .grant
        .tenant_id;

        let widened = profile.with_roles(vec!["reader".to_string()]);
        assert_eq!(count(&widened.with_federated()), 0);
        assert_eq!(count(&widened.from_issuer(issuer)), 0);
        assert_eq!(
            count(&profile.with_federated().on_tenant(federated_tenant)),
            1
        );
    }
}
