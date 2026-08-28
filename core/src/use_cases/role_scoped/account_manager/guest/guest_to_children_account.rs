use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::VerboseStatus, account_type::AccountType, email::Email,
            guest_user::GuestUser, native_error_codes::NativeErrorCodes,
            profile::Profile,
        },
        entities::{
            AccountFetching, GuestRoleFetching, GuestUserRegistration,
            LocalMessageWrite, TenantFetching,
        },
    },
    models::AccountLifeCycle,
    settings::DEFAULT_TENANT_ID_KEY,
    use_cases::support::dispatch_notification,
};

use futures::future;
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Guest users to collaborate to an account under a role which I can delegate.
///
/// A role is delegable when the requester holds its parent role, or — when the
/// role is a root, having no parent at all — when the requester already holds
/// the role itself on the target account. Either way nobody grants what they
/// do not have.
///
/// This action should be allowed only to accounts that contains registered
/// children accounts already registered.
#[tracing::instrument(name = "guest_to_children_account", skip_all)]
pub async fn guest_to_children_account(
    profile: Profile,
    tenant_id: Uuid,
    email: Email,
    target_role_id: Uuid,
    target_account_id: Uuid,
    life_cycle_settings: AccountLifeCycle,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .on_account(target_account_id)
        .with_write_access()
        .with_roles(vec![SystemActor::AccountManager])
        .get_related_account_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Guarantee needed information to evaluate guesting
    //
    // Check if the target account is a subscription account or a standard role
    // associated account. Only these accounts can receive guesting. Already
    // check the role_id to be a guest role is valid and exists.
    //
    // ? -----------------------------------------------------------------------

    let (target_account_response, parent_role_response, target_role_response) =
        future::join3(
            account_fetching_repo.get(target_account_id, related_accounts),
            //
            // Get the parent role by child id to ensure the target role is a
            // child of the parent role.
            //
            guest_role_fetching_repo.get_parent_by_child_id(target_role_id),
            guest_role_fetching_repo.get(target_role_id),
        )
        .await;

    let target_account = match target_account_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Target account not found: {:?}",
                id.unwrap()
            ))
            .with_exp_true()
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
        FetchResponseKind::Found(account) => match account.account_type {
            AccountType::Subscription { .. }
            | AccountType::RoleAssociated { .. } => account,
            _ => return use_case_err(
                "Invalid account. Only subscription accounts should receive \
                guesting.",
            )
            .as_error(),
        },
    };

    if let Some(status) = target_account.verbose_status {
        if status != VerboseStatus::Verified {
            return use_case_err(
                "Invalid account status. Only active accounts \
                should receive guesting.",
            )
            .as_error();
        }
    } else {
        return use_case_err(
            "Unable to check account status for guesting. \
            Account is maybe inactive.",
        )
        .as_error();
    }

    //
    // Resolve the target role before branching on the hierarchy. A role that
    // does not exist has no parent either, so resolving the parent first would
    // route a missing role into the root path and report it as a missing
    // license instead of a missing role.
    //
    let target_role = match target_role_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Guest role not found: {:?}",
                id.unwrap()
            ))
            .with_exp_true()
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
        FetchResponseKind::Found(role) => role,
    };

    tracing::debug!("target_role: {:?}", target_role);
    tracing::debug!("profile: {:?}", profile);

    match parent_role_response? {
        //
        // The target role has a parent. Use the parent role to verify if the
        // user that perform this action has permission to access the child
        // role.
        //
        FetchResponseKind::Found(parent_role) => {
            tracing::debug!("parent_role: {:?}", parent_role);

            let parent_role_id = if let Some(id) = parent_role.id {
                id
            } else {
                return use_case_err(
                    "Invalid parent role. Only roles with guest to collaborate can receive guesting",
                )
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
            };

            //
            // Verify if the current role has guest to collaborate to the
            // parent role
            //
            if let Some(resources) = profile.licensed_resources.as_ref() {
                if !resources
                    .to_licenses_vector()
                    .iter()
                    .any(|i| i.role_id == parent_role_id)
                {
                    return use_case_err(
                        "You are not allowed to perform this action. You must be a guest to the parent role to collaborate to the child role.",
                    )
                    .with_exp_true()
                    .with_code(NativeErrorCodes::MYC00013)
                    .as_error();
                }
            } else {
                return use_case_err(
                    "You are not allowed to perform this action. You must have a guest role to collaborate to the parent role.",
                )
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
            }

            //
            // Verify if the target role is a child of the parent role.
            //
            if let Some(children) = parent_role.children {
                let target_ids = match children {
                    Children::Ids(ids) => ids,
                    Children::Records(records) => records
                        .iter()
                        .filter_map(|i| i.id)
                        .collect::<Vec<Uuid>>(),
                };

                if !target_ids.contains(&target_role_id) {
                    return use_case_err(
                        "Invalid target role. Children role is not belong to \
                        the parent role",
                    )
                    .with_exp_true()
                    .with_code(NativeErrorCodes::MYC00013)
                    .as_error();
                }
            } else {
                return use_case_err(
                    "Invalid parent role. Only roles with children can \
                    receive guesting",
                )
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
            }
        }
        //
        // The target role is a root role. Root roles have no parent to
        // delegate from, so the requester must already hold the role itself.
        // The check runs against the profile scoped to the tenant and the
        // target account, otherwise holding the role on one account would
        // authorize granting it on another.
        //
        FetchResponseKind::NotFound(_) => {
            let holds_target_role = profile
                .on_tenant(tenant_id)
                .on_account(target_account_id)
                .licensed_resources
                .map(|resources| {
                    resources
                        .to_licenses_vector()
                        .iter()
                        .any(|i| i.role_id == target_role_id)
                })
                .unwrap_or(false);

            if !holds_target_role {
                return use_case_err(
                    "You are not allowed to perform this action. The target role has no parent role, then you must already hold it on the target account to grant it.",
                )
                .with_exp_true()
                .with_code(NativeErrorCodes::MYC00013)
                .as_error();
            }
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    let guest_user = match guest_user_registration_repo
        .get_or_create(
            GuestUser::new_unverified(
                email.to_owned(),
                Parent::Id(target_role_id),
                None,
            ),
            target_account_id,
        )
        .await
    {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!("Unable to create guest user: {err}"))
                .with_code(NativeErrorCodes::MYC00017)
                .with_exp_true()
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    if let Err(err) = dispatch_notification(
        vec![
            ("account_name", target_account.name.to_uppercase()),
            ("role_name", target_role.name.to_uppercase()),
            ("role_permissions", target_role.permission.to_string()),
            (DEFAULT_TENANT_ID_KEY, tenant_id.to_string()),
        ],
        "email/guest-to-subscription-account",
        life_cycle_settings,
        email,
        None,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Send the guesting response
    // ? -----------------------------------------------------------------------

    Ok(guest_user)
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::domain::dtos::{
        account::Account,
        guest_role::{GuestRole, Permission},
        message::MessageSendingEvent,
        profile::{LicensedResource, LicensedResources},
        related_accounts::RelatedAccounts,
        telegram::TelegramUserId,
        tenant::{Tenant, TenantMetaKey},
    };
    use crate::models::{HmacSecretEntry, HmacSecretSet};

    use async_trait::async_trait;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::{CreateResponseKind, FetchManyResponseKind};

    // ? -----------------------------------------------------------------------
    // ? Fixtures
    // ? -----------------------------------------------------------------------

    fn config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("Test Domain".to_string()),
            domain_url: Some(SecretResolver::Value(
                "https://test.com".to_string(),
            )),
            locale: Some(SecretResolver::Value("en-us".to_string())),
            token_expiration: SecretResolver::Value(3600),
            noreply_name: Some(SecretResolver::Value(
                "Test System".to_string(),
            )),
            noreply_email: SecretResolver::Value(
                "noreply@test.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@test.com".to_string(),
            ),
            token_secret: SecretResolver::Value("test-secret".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("test-hmac".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    fn guest_role(
        id: Uuid,
        name: &str,
        children: Option<Children<GuestRole, Uuid>>,
    ) -> GuestRole {
        GuestRole::new(
            Some(id),
            name.to_string(),
            None,
            Permission::Write,
            children,
            false,
        )
    }

    fn license(
        tenant_id: Uuid,
        acc_id: Uuid,
        role_id: Uuid,
        role: &str,
    ) -> LicensedResource {
        LicensedResource {
            acc_id,
            tenant_id,
            role_id,
            acc_name: "Target Account".to_string(),
            sys_acc: false,
            role: role.to_string(),
            perm: Permission::Write,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }
    }

    fn profile_with(licenses: Vec<LicensedResource>) -> Profile {
        let mut profile = Profile::default();

        profile.licensed_resources = Some(LicensedResources::Records(licenses));

        profile
    }

    // ? -----------------------------------------------------------------------
    // ? Stubs
    // ? -----------------------------------------------------------------------

    struct StubAccountFetching {
        account: Account,
    }

    #[async_trait]
    impl AccountFetching for StubAccountFetching {
        async fn get(
            &self,
            _: Uuid,
            _: RelatedAccounts,
        ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::Found(self.account.to_owned()))
        }

        async fn list(
            &self,
            _: RelatedAccounts,
            _: Option<String>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<bool>,
            _: Option<Uuid>,
            _: Option<String>,
            _: Option<Uuid>,
            _: AccountType,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
            unimplemented!()
        }

        async fn get_by_telegram_id(
            &self,
            _: TelegramUserId,
        ) -> Result<FetchResponseKind<Account, i64>, MappedErrors> {
            unimplemented!()
        }
    }

    struct StubGuestRoleFetching {
        target_role: Option<GuestRole>,
        parent_role: Option<GuestRole>,
    }

    #[async_trait]
    impl GuestRoleFetching for StubGuestRoleFetching {
        async fn get(
            &self,
            id: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            Ok(match self.target_role.to_owned() {
                Some(role) => FetchResponseKind::Found(role),
                None => FetchResponseKind::NotFound(Some(id)),
            })
        }

        async fn get_parent_by_child_id(
            &self,
            id: Uuid,
        ) -> Result<FetchResponseKind<GuestRole, Uuid>, MappedErrors> {
            Ok(match self.parent_role.to_owned() {
                Some(role) => FetchResponseKind::Found(role),
                None => FetchResponseKind::NotFound(Some(id)),
            })
        }

        async fn list(
            &self,
            _: Option<String>,
            _: Option<String>,
            _: Option<bool>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<GuestRole>, MappedErrors> {
            unimplemented!()
        }
    }

    struct StubGuestUserRegistration;

    #[async_trait]
    impl GuestUserRegistration for StubGuestUserRegistration {
        async fn get_or_create(
            &self,
            guest_user: GuestUser,
            _: Uuid,
        ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
            Ok(GetOrCreateResponseKind::Created(guest_user))
        }
    }

    struct StubLocalMessageWrite;

    #[async_trait]
    impl LocalMessageWrite for StubLocalMessageWrite {
        async fn send(
            &self,
            _: MessageSendingEvent,
        ) -> Result<CreateResponseKind<Option<Uuid>>, MappedErrors> {
            Ok(CreateResponseKind::Created(Some(Uuid::new_v4())))
        }

        async fn update_message_event(
            &self,
            _: MessageSendingEvent,
        ) -> Result<(), MappedErrors> {
            unimplemented!()
        }

        async fn delete_message_event(
            &self,
            _: Uuid,
        ) -> Result<(), MappedErrors> {
            unimplemented!()
        }

        async fn ping(&self) -> Result<(), MappedErrors> {
            unimplemented!()
        }
    }

    struct StubTenantFetching;

    #[async_trait]
    impl TenantFetching for StubTenantFetching {
        async fn get_tenant_owned_by_me(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn get_tenant_public_by_id(
            &self,
            _: Uuid,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            Ok(FetchResponseKind::NotFound(None))
        }

        async fn get_tenants_by_manager_account(
            &self,
            _: Uuid,
            _: Vec<Uuid>,
        ) -> Result<FetchResponseKind<Tenant, String>, MappedErrors> {
            unimplemented!()
        }

        async fn filter_tenants_as_manager(
            &self,
            _: Option<String>,
            _: Option<Uuid>,
            _: Option<(TenantMetaKey, String)>,
            _: Option<(String, String)>,
            _: Option<i32>,
            _: Option<i32>,
        ) -> Result<FetchManyResponseKind<Tenant>, MappedErrors> {
            unimplemented!()
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Runner
    // ? -----------------------------------------------------------------------

    async fn run(
        profile: Profile,
        tenant_id: Uuid,
        target_account_id: Uuid,
        target_role_id: Uuid,
        target_role: Option<GuestRole>,
        parent_role: Option<GuestRole>,
    ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
        let account_fetching_repo = StubAccountFetching {
            account: Account {
                id: Some(target_account_id),
                name: "Target Account".to_string(),
                verbose_status: Some(VerboseStatus::Verified),
                account_type: AccountType::Subscription { tenant_id },
                ..Default::default()
            },
        };

        let guest_role_fetching_repo = StubGuestRoleFetching {
            target_role,
            parent_role,
        };

        let guest_user_registration_repo = StubGuestUserRegistration;
        let message_sending_repo = StubLocalMessageWrite;
        let tenant_fetching_repo = StubTenantFetching;

        guest_to_children_account(
            profile,
            tenant_id,
            Email::from_string("guest@example.com".to_string())
                .expect("valid email"),
            target_role_id,
            target_account_id,
            config(),
            Box::new(&account_fetching_repo),
            Box::new(&guest_role_fetching_repo),
            Box::new(&guest_user_registration_repo),
            Box::new(&message_sending_repo),
            Box::new(&tenant_fetching_repo),
        )
        .await
    }

    // ? -----------------------------------------------------------------------
    // ? Root (parentless) role path
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn root_role_is_granted_when_requester_holds_it() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();

        let profile = profile_with(vec![
            license(
                tenant_id,
                target_account_id,
                Uuid::new_v4(),
                SystemActor::AccountManager.str(),
            ),
            license(tenant_id, target_account_id, target_role_id, "customer"),
        ]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            Some(guest_role(target_role_id, "Customer", None)),
            None,
        )
        .await;

        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[tokio::test]
    async fn root_role_is_denied_when_requester_does_not_hold_it() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();

        let profile = profile_with(vec![license(
            tenant_id,
            target_account_id,
            Uuid::new_v4(),
            SystemActor::AccountManager.str(),
        )]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            Some(guest_role(target_role_id, "Customer", None)),
            None,
        )
        .await;

        let err = result.expect_err("expected an unauthorized error");

        assert!(err.has_str_code("MYC00013"));
        assert!(err.msg().contains("has no parent role"));
    }

    #[tokio::test]
    async fn root_role_is_denied_when_held_on_another_account() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let another_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();

        let profile = profile_with(vec![
            license(
                tenant_id,
                target_account_id,
                Uuid::new_v4(),
                SystemActor::AccountManager.str(),
            ),
            //
            // The role is held, but on a different account of the same tenant.
            //
            license(tenant_id, another_account_id, target_role_id, "customer"),
        ]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            Some(guest_role(target_role_id, "Customer", None)),
            None,
        )
        .await;

        let err = result.expect_err("expected an unauthorized error");

        assert!(err.has_str_code("MYC00013"));
        assert!(err.msg().contains("has no parent role"));
    }

    // ? -----------------------------------------------------------------------
    // ? Child role path
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn child_role_is_granted_when_requester_holds_the_parent() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();
        let parent_role_id = Uuid::new_v4();

        let profile = profile_with(vec![
            license(
                tenant_id,
                target_account_id,
                Uuid::new_v4(),
                SystemActor::AccountManager.str(),
            ),
            license(tenant_id, target_account_id, parent_role_id, "manager"),
        ]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            Some(guest_role(target_role_id, "Customer", None)),
            Some(guest_role(
                parent_role_id,
                "Manager",
                Some(Children::Ids(vec![target_role_id])),
            )),
        )
        .await;

        assert!(result.is_ok(), "unexpected error: {:?}", result.err());
    }

    #[tokio::test]
    async fn child_role_is_denied_when_requester_does_not_hold_the_parent() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();
        let parent_role_id = Uuid::new_v4();

        let profile = profile_with(vec![license(
            tenant_id,
            target_account_id,
            Uuid::new_v4(),
            SystemActor::AccountManager.str(),
        )]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            Some(guest_role(target_role_id, "Customer", None)),
            Some(guest_role(
                parent_role_id,
                "Manager",
                Some(Children::Ids(vec![target_role_id])),
            )),
        )
        .await;

        let err = result.expect_err("expected an unauthorized error");

        assert!(err.has_str_code("MYC00013"));
        assert!(err.msg().contains("guest to the parent role"));
    }

    #[tokio::test]
    async fn child_role_is_denied_when_not_listed_in_the_parent_children() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();
        let parent_role_id = Uuid::new_v4();

        let profile = profile_with(vec![
            license(
                tenant_id,
                target_account_id,
                Uuid::new_v4(),
                SystemActor::AccountManager.str(),
            ),
            license(tenant_id, target_account_id, parent_role_id, "manager"),
        ]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            Some(guest_role(target_role_id, "Customer", None)),
            Some(guest_role(
                parent_role_id,
                "Manager",
                Some(Children::Ids(vec![Uuid::new_v4()])),
            )),
        )
        .await;

        let err = result.expect_err("expected an unauthorized error");

        assert!(err.has_str_code("MYC00013"));
        assert!(err.msg().contains("is not belong to the parent role"));
    }

    // ? -----------------------------------------------------------------------
    // ? Ordering
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn missing_target_role_is_reported_before_the_parent_branch() {
        let tenant_id = Uuid::new_v4();
        let target_account_id = Uuid::new_v4();
        let target_role_id = Uuid::new_v4();

        let profile = profile_with(vec![license(
            tenant_id,
            target_account_id,
            Uuid::new_v4(),
            SystemActor::AccountManager.str(),
        )]);

        let result = run(
            profile,
            tenant_id,
            target_account_id,
            target_role_id,
            None,
            None,
        )
        .await;

        let err = result.expect_err("expected a not found error");

        assert!(err.has_str_code("MYC00013"));
        assert!(err.msg().contains("Guest role not found"));
    }
}
