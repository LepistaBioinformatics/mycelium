use crate::domain::dtos::{federation::SubjectStanding, profile::Profile};

/// Why the local profile must gain nothing from federation, if it must not
/// (FED-19)
///
/// Federation adds grants; it never overrides a local denial. An inactive
/// owner, or an inactive, archived or deleted account, gets no federated
/// grant whatever its peers assert.
///
pub fn locally_denied(profile: &Profile) -> Option<String> {
    SubjectStanding::from(profile).denial()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_active_account_is_not_denied() {
        assert_eq!(locally_denied(&Profile::default()), None);
    }

    #[test]
    fn each_local_denial_is_named() {
        let cases: [(fn(&mut Profile), &str); 4] = [
            (
                |profile| profile.owner_is_active = false,
                "owner is inactive",
            ),
            (
                |profile| profile.account_is_active = false,
                "account is inactive",
            ),
            (|profile| profile.account_was_archived = true, "archived"),
            (|profile| profile.account_was_deleted = true, "deleted"),
        ];

        for (deny, reason) in cases {
            let mut profile = Profile::default();
            deny(&mut profile);

            let denial = locally_denied(&profile).unwrap();
            assert!(denial.contains(reason), "{denial}");
        }
    }
}
