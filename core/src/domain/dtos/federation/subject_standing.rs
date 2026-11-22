use crate::domain::dtos::profile::Profile;

/// The local state that decides whether a subject may hold any federated
/// grant, on either side of a link (FED-19)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubjectStanding {
    pub owner_is_active: bool,
    pub account_is_active: bool,
    pub account_was_archived: bool,
    pub account_was_deleted: bool,
}

impl SubjectStanding {
    /// Why the subject gains nothing from federation, if it must not
    pub fn denial(&self) -> Option<String> {
        let denials = [
            (!self.owner_is_active, "the account owner is inactive"),
            (!self.account_is_active, "the account is inactive"),
            (self.account_was_archived, "the account is archived"),
            (self.account_was_deleted, "the account is deleted"),
        ];

        denials
            .into_iter()
            .find(|(denied, _)| *denied)
            .map(|(_, reason)| reason.to_string())
    }
}

impl From<&Profile> for SubjectStanding {
    fn from(profile: &Profile) -> Self {
        Self {
            owner_is_active: profile.owner_is_active,
            account_is_active: profile.account_is_active,
            account_was_archived: profile.account_was_archived,
            account_was_deleted: profile.account_was_deleted,
        }
    }
}
