use super::RoleCatalogEntry;

use serde::{Deserialize, Serialize};

/// The payload of a `guest-role-vocabulary.v1` assertion: the issuer's guest
/// roles, with no tenant or account id
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleVocabularyAssertion {
    pub roles: Vec<RoleCatalogEntry>,
}

/// What a peer answered when asked for its vocabulary
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum PeerVocabularyAnswer {
    /// A verified assertion of the vocabulary kind
    Verified { roles: Vec<RoleCatalogEntry> },

    /// The peer does not serve it now (catalog off, kind not negotiated) or
    /// answered another kind, which is ignored (FED-41)
    Unknown,

    /// An assertion came back but failed verification
    Rejected { reason: String },
}
