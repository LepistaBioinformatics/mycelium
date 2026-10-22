use crate::domain::dtos::email::Email;

use serde::{Deserialize, Serialize};

/// Issued when the emailed code is verified: the client exchanges `code`
/// once at the token endpoint for a session of `email`'s user.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostedLoginAuthorization {
    pub code: String,
    pub email: Email,
}
