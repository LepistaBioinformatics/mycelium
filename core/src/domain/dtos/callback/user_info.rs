use crate::domain::dtos::{email::Email, profile::Profile};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UserInfo {
    Email(Email),
    Profile(Profile),
}

impl UserInfo {
    pub fn new_email(email: Email) -> Self {
        Self::Email(email)
    }

    pub fn new_profile(profile: Profile) -> Self {
        Self::Profile(profile)
    }
}
