use super::LicensedResource;

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;

/// A grant the subject holds on a peer instance (FED-27)
///
/// Carried apart from the local `licensedResources`, so a filter never takes
/// a foreign grant for a local one unless asked to (FED-31, I-2). Its tenant
/// and account ids are the issuer's, guarded against collisions with this
/// instance's own (I-1).
///
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FederatedResource {
    /// The instance id of the peer that asserted the grant
    pub issuer: Uuid,

    #[serde(flatten)]
    pub grant: LicensedResource,
}

/// Every federated grant of a profile
#[derive(
    Clone, Debug, Default, Deserialize, Serialize, ToSchema, PartialEq,
)]
#[serde(transparent)]
pub struct FederatedResources(pub Vec<FederatedResource>);

impl FederatedResources {
    /// The grants, whatever their issuer
    pub fn grants(&self) -> Vec<LicensedResource> {
        self.0
            .iter()
            .map(|resource| resource.grant.clone())
            .collect()
    }

    /// The grants asserted by one issuer
    pub fn grants_from(&self, issuer: Uuid) -> Vec<LicensedResource> {
        self.0
            .iter()
            .filter(|resource| resource.issuer == issuer)
            .map(|resource| resource.grant.clone())
            .collect()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}
