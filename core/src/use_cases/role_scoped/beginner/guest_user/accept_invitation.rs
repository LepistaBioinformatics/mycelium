use crate::domain::{
    dtos::{
        email::Email,
        guest_role::Permission,
        native_error_codes::NativeErrorCodes,
        profile::{LicensedResource, Profile},
    },
    entities::GuestUserOnAccountUpdating,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Accept invitation to join an account
///
/// After a guest user has been invited to join an account, they can accept the
/// invitation to join the account. Only the caller's own pending invitation is
/// verified: the guest is matched by the profile owner's email, the account and
/// the role of the pending license.
///
#[tracing::instrument(name = "accept_invitation", skip_all)]
pub async fn accept_invitation(
    profile: Profile,
    account_id: Uuid,
    role_name: String,
    permission: Permission,
    guest_user_on_account_repo: Box<&dyn GuestUserOnAccountUpdating>,
) -> Result<UpdatingResponseKind<(String, Uuid, Permission)>, MappedErrors> {
    let license =
        find_pending_license(&profile, account_id, &role_name, &permission)?;

    let guest_emails = profile
        .owners
        .iter()
        .map(|owner| Email::from_string(owner.email.to_owned()))
        .collect::<Result<Vec<Email>, MappedErrors>>()?;

    let invitation = (role_name, account_id, permission);

    match guest_user_on_account_repo
        .accept_invitation(guest_emails, account_id, license.role_id)
        .await?
    {
        UpdatingResponseKind::Updated(_) => {
            Ok(UpdatingResponseKind::Updated(invitation))
        }
        UpdatingResponseKind::NotUpdated(_, reason) => {
            Ok(UpdatingResponseKind::NotUpdated(invitation, reason))
        }
    }
}

/// The unverified license of the profile matching the invitation parameters
fn find_pending_license(
    profile: &Profile,
    account_id: Uuid,
    role_name: &str,
    permission: &Permission,
) -> Result<LicensedResource, MappedErrors> {
    let Some(licensed_resources) = profile.licensed_resources.as_ref() else {
        return use_case_err("Profile does not have an account id")
            .with_exp_true()
            .as_error();
    };

    licensed_resources
        .to_licenses_vector()
        .into_iter()
        .find(|license| {
            license.acc_id == account_id
                && license.role == role_name
                && &license.perm == permission
                && !license.verified
        })
        .ok_or_else(|| {
            use_case_err(
                "Invalid operation. License does not match the invitation \
                 parameters",
            )
            .with_code(NativeErrorCodes::MYC00018)
            .with_exp_true()
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::profile::{LicensedResources, Owner},
        entities::MockGuestUserOnAccountUpdating,
    };
    use mockall::predicate::eq;

    const ROLE: &str = "rc-manager";

    fn license(
        acc_id: Uuid,
        role_id: Uuid,
        verified: bool,
    ) -> LicensedResource {
        LicensedResource {
            acc_id,
            sys_acc: false,
            tenant_id: Uuid::new_v4(),
            acc_name: "Account".to_string(),
            role: ROLE.to_string(),
            role_id,
            perm: Permission::Write,
            verified,
            permit_flags: None,
            deny_flags: None,
        }
    }

    fn profile(email: &str, licenses: Vec<LicensedResource>) -> Profile {
        let mut profile = Profile::new(
            vec![Owner {
                id: Uuid::new_v4(),
                email: email.to_string(),
                first_name: None,
                last_name: None,
                username: None,
                is_principal: true,
            }],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        );
        profile.licensed_resources = Some(LicensedResources::Records(licenses));
        profile
    }

    #[tokio::test]
    async fn verifies_only_the_callers_guest_on_the_pending_role(
    ) -> Result<(), MappedErrors> {
        let account_id = Uuid::new_v4();
        let verified_role = Uuid::new_v4();
        let pending_role = Uuid::new_v4();
        let caller = profile(
            "bob@example.com",
            vec![
                license(Uuid::new_v4(), verified_role, true),
                license(account_id, pending_role, false),
            ],
        );

        let mut repo = MockGuestUserOnAccountUpdating::new();
        repo.expect_accept_invitation()
            .with(
                eq(vec![Email::from_string("bob@example.com".into())?]),
                eq(account_id),
                eq(pending_role),
            )
            .times(1)
            .returning(|_, _, _| {
                Ok(UpdatingResponseKind::Updated(Uuid::new_v4()))
            });

        let result = accept_invitation(
            caller,
            account_id,
            ROLE.to_string(),
            Permission::Write,
            Box::new(&repo),
        )
        .await?;

        assert!(matches!(
            result,
            UpdatingResponseKind::Updated((role, acc, Permission::Write))
                if role == ROLE && acc == account_id
        ));

        Ok(())
    }

    #[tokio::test]
    async fn rejects_when_no_pending_license_matches() {
        let account_id = Uuid::new_v4();
        let caller = profile(
            "bob@example.com",
            vec![license(account_id, Uuid::new_v4(), true)],
        );

        let mut repo = MockGuestUserOnAccountUpdating::new();
        repo.expect_accept_invitation().never();

        let result = accept_invitation(
            caller,
            account_id,
            ROLE.to_string(),
            Permission::Write,
            Box::new(&repo),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn reports_not_updated_when_the_guest_is_not_found(
    ) -> Result<(), MappedErrors> {
        let account_id = Uuid::new_v4();
        let caller = profile(
            "bob@example.com",
            vec![license(account_id, Uuid::new_v4(), false)],
        );

        let mut repo = MockGuestUserOnAccountUpdating::new();
        repo.expect_accept_invitation().returning(|_, _, _| {
            Ok(UpdatingResponseKind::NotUpdated(
                Uuid::nil(),
                "No unverified guest user found for account".to_string(),
            ))
        });

        let result = accept_invitation(
            caller,
            account_id,
            ROLE.to_string(),
            Permission::Write,
            Box::new(&repo),
        )
        .await?;

        assert!(matches!(result, UpdatingResponseKind::NotUpdated(..)));

        Ok(())
    }
}
