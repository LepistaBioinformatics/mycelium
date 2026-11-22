use crate::domain::dtos::profile::FederatedResources;

/// The federated grants of a subject as the store holds them now (D-6)
#[derive(Clone, Debug, Default, PartialEq)]
pub struct StoredFederatedGrants {
    pub resources: FederatedResources,

    /// Some trusted, linked peer has no stored assertion or a due one
    pub refresh_due: bool,

    /// The peers whose stored assertions contributed (their translation
    /// rules apply)
    pub peer_ids: Vec<uuid::Uuid>,
}
