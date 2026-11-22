use super::SubjectRef;

use serde::{Deserialize, Serialize};

/// What a consumer sends a peer to resolve a subject (FED-12)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolutionRequest {
    /// The peer's `local_subject_ref` of the link
    pub subject_ref: SubjectRef,
}

/// A peer's answer to a resolution request
///
/// `Unknown` covers every reference without an active link with the caller,
/// so the answer reveals nothing else (R-9).
///
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
pub enum ResolutionAnswer {
    Assertion { jws: String },
    Unknown,
}
