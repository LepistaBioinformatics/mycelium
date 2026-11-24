use super::SubjectRef;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// What the issuer tells the consumer once the user confirmed a link
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkConfirmation {
    /// The consumer's link request, the `jti` of the redirect state
    pub request_id: Uuid,

    /// How the consumer names the user when asking the issuer
    pub issuer_subject_ref: SubjectRef,
}

/// The consumer's answer to a confirmation
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LinkAcknowledgement {
    /// How the issuer names the user when asking the consumer
    pub consumer_subject_ref: SubjectRef,
}
