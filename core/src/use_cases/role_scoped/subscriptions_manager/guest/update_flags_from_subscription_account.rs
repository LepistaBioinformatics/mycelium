use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::VerboseStatus, account_type::AccountType,
        guest_role::Permission, guest_user_on_account::GuestUserOnAccount,
        native_error_codes::NativeErrorCodes, profile::Profile,
    },
    entities::{
        AccountFetching, GuestUserOnAccountFetching, GuestUserOnAccountUpdating,
    },
};

use futures::future;
use mycelium_base::{
    entities::{
        FetchManyResponseKind, FetchResponseKind, UpdatingResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Update permit and deny flags for a guest user
///
/// This action is restricted to subscription accounts and only subscription
/// managers and tenant managers can perform this action. Permit and deny flags
/// provided should be inserted or removed from the guest user on account
/// record.
///
#[tracing::instrument(
    name = "update_flags_from_subscription_account",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn update_flags_from_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    guest_role_id: Uuid,
    account_id: Uuid,
    permit_flags: Vec<String>,
    deny_flags: Vec<String>,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_on_account_fetching_repo: Box<&dyn GuestUserOnAccountFetching>,
    guest_user_on_account_updating_repo: Box<&dyn GuestUserOnAccountUpdating>,
) -> Result<UpdatingResponseKind<GuestUserOnAccount>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![
            SystemActor::TenantManager,
            SystemActor::SubscriptionsManager,
        ])
        .get_related_accounts_or_tenant_wide_permission_or_error(
            tenant_id,
            Permission::Write,
        )?;

    // ? -----------------------------------------------------------------------
    // ? Fetch the target account and guest user on account list
    // ? -----------------------------------------------------------------------

    let (target_account_response, guest_user_on_account_list_response) =
        future::join(
            account_fetching_repo.get(account_id, related_accounts),
            guest_user_on_account_fetching_repo
                .list_by_guest_role_id(guest_role_id, account_id),
        )
        .await;

    // ? -----------------------------------------------------------------------
    // ? Validate the target account
    //
    // The target account must be a subscription account and must be verified.
    //
    // ? -----------------------------------------------------------------------

    let target_account = match target_account_response? {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Target account not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
    };

    let account_id = match target_account.id {
        Some(id) => id,
        None => {
            return use_case_err(
                "Unable to find account id. This should never happen.",
            )
            .as_error()
        }
    };

    match target_account.account_type {
        AccountType::Subscription { .. } => {
            profile
                .on_account(account_id)
                .with_write_access()
                .with_roles(vec![SystemActor::SubscriptionsManager])
                .get_related_account_or_error()?;
        }
        _ => {
            return use_case_err(
                "Invalid account. Only subscription accounts should update permit and deny flags.",
            )
            .as_error()
        }
    };

    if let Some(status) = target_account.verbose_status {
        if status != VerboseStatus::Verified {
            return use_case_err(
                "Invalid account status. Only active accounts should update permit and deny flags.",
            )
            .as_error();
        }
    } else {
        return use_case_err(
            "Unable to check account status for updating permit and deny flags. Account is maybe inactive.",
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Validate the guest user on account list
    //
    // The guest user on account list must not be empty and must contain only
    // one record.
    //
    // ? -----------------------------------------------------------------------

    let mut guest_user_on_account = match guest_user_on_account_list_response? {
        FetchManyResponseKind::Found(records) => {
            if records.len() > 1 {
                tracing::error!("Invalid operation. Multiple guest users on account found for the given guest role id and account id.");

                return use_case_err("Invalid operation. Operation restricted to single guest user on account. Please contact support.")
                    .with_code(NativeErrorCodes::MYC00018)
                    .with_exp_true()
                    .as_error()
            }

            records.first().cloned().ok_or_else(|| {
                use_case_err("No guest user on account found for the given guest role id and account id.")
                    .with_code(NativeErrorCodes::MYC00018)
                    .with_exp_true()
            })?
        },
        FetchManyResponseKind::FoundPaginated { .. } => {
            tracing::error!("Paginated response is not supported on update permit and deny flags to subscription account.");

            return use_case_err("Internal error on update permit and deny flags to subscription account. Contact support.")
                .as_error()
        }
        FetchManyResponseKind::NotFound => {
            return use_case_err("No guest user on account found for the given guest role id and account id.")
                .with_code(NativeErrorCodes::MYC00018)
                .with_exp_true()
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Update permit flags and deny flags
    // ? -----------------------------------------------------------------------

    guest_user_on_account.permit_flags = permit_flags;
    guest_user_on_account.deny_flags = deny_flags;

    // ? -----------------------------------------------------------------------
    // ? Update guest user on account
    // ? -----------------------------------------------------------------------

    guest_user_on_account_updating_repo
        .update(guest_user_on_account)
        .await
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            account::{Account, VerboseStatus},
            account_type::AccountType,
            guest_role::Permission,
            profile::{LicensedResource, LicensedResources, Profile},
            related_accounts::RelatedAccounts,
        },
        entities::{AccountFetching, GuestUserOnAccountFetching},
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::{
        FetchManyResponseKind, FetchResponseKind, UpdatingResponseKind,
    };
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Helper functions
    // ? -----------------------------------------------------------------------

    fn create_subscription_account(
        account_id: Uuid,
        tenant_id: Uuid,
        verbose_status: Option<VerboseStatus>,
    ) -> Account {
        let mut account = Account::new_subscription_account(
            "Test Subscription Account".to_string(),
            tenant_id,
            None,
        );
        account.id = Some(account_id);
        account.verbose_status = verbose_status;
        account
    }

    fn create_guest_user_on_account(
        guest_user_id: Uuid,
        account_id: Uuid,
        permit_flags: Vec<String>,
        deny_flags: Vec<String>,
    ) -> GuestUserOnAccount {
        GuestUserOnAccount {
            guest_user_id,
            account_id,
            created: Local::now(),
            permit_flags,
            deny_flags,
        }
    }

    fn create_profile_with_permissions(
        tenant_id: Uuid,
        account_id: Uuid,
        role_id: Uuid,
        role_name: &str,
        permission: Permission,
    ) -> Profile {
        use crate::domain::actors::SystemActor;

        // Convert role name to the correct string format
        let role_string = match role_name {
            "SubscriptionsManager" => SystemActor::SubscriptionsManager.str(),
            "TenantManager" => SystemActor::TenantManager.str(),
            _ => role_name,
        };

        Profile::new(
            vec![],
            account_id,
            true,  // is_subscription
            false, // is_manager
            false, // is_staff
            true,  // owner_is_active
            true,  // account_is_active
            true,  // account_was_approved
            false, // account_was_archived
            false, // account_was_deleted
            None,  // verbose_status
            Some(LicensedResources::Records(vec![LicensedResource {
                acc_id: account_id,
                tenant_id,
                role_id,
                acc_name: "Test Account".to_string(),
                sys_acc: true, // System accounts for SubscriptionsManager/TenantManager
                role: role_string.to_string(),
                perm: permission,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            }])),
            None, // tenants_ownership
        )
    }

    // ? -----------------------------------------------------------------------
    // ? Mock repositories
    // ? -----------------------------------------------------------------------

    struct MockAccountFetching {
        account: Option<Account>,
        should_fail: bool,
    }

    impl MockAccountFetching {
        fn with_account(account: Account) -> Self {
            Self {
                account: Some(account),
                should_fail: false,
            }
        }

        fn not_found() -> Self {
            Self {
                account: None,
                should_fail: false,
            }
        }
    }

    #[async_trait]
    impl AccountFetching for MockAccountFetching {
        async fn get(
            &self,
            _: Uuid,
            _: RelatedAccounts,
        ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
            if self.should_fail {
                return use_case_err("Database error".to_string()).as_error();
            }

            match &self.account {
                Some(account) => Ok(FetchResponseKind::Found(account.clone())),
                None => Ok(FetchResponseKind::NotFound(Some(Uuid::new_v4()))),
            }
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
    }

    struct MockGuestUserOnAccountFetching {
        records: Vec<GuestUserOnAccount>,
        should_fail: bool,
        return_paginated: bool,
    }

    impl MockGuestUserOnAccountFetching {
        fn with_single_record(record: GuestUserOnAccount) -> Self {
            Self {
                records: vec![record],
                should_fail: false,
                return_paginated: false,
            }
        }

        fn with_multiple_records(records: Vec<GuestUserOnAccount>) -> Self {
            Self {
                records,
                should_fail: false,
                return_paginated: false,
            }
        }

        fn with_paginated_response(records: Vec<GuestUserOnAccount>) -> Self {
            Self {
                records,
                should_fail: false,
                return_paginated: true,
            }
        }

        fn not_found() -> Self {
            Self {
                records: vec![],
                should_fail: false,
                return_paginated: false,
            }
        }
    }

    #[async_trait]
    impl GuestUserOnAccountFetching for MockGuestUserOnAccountFetching {
        async fn list_by_guest_role_id(
            &self,
            _: Uuid,
            _: Uuid,
        ) -> Result<FetchManyResponseKind<GuestUserOnAccount>, MappedErrors>
        {
            if self.should_fail {
                return use_case_err("Database error".to_string()).as_error();
            }

            if self.should_fail {
                return use_case_err("Database error".to_string()).as_error();
            }

            if self.records.is_empty() {
                Ok(FetchManyResponseKind::NotFound)
            } else if self.return_paginated {
                Ok(FetchManyResponseKind::FoundPaginated {
                    count: self.records.len() as i64,
                    skip: None,
                    size: None,
                    records: self.records.clone(),
                })
            } else {
                // Always return Found, even for multiple records, so the use case can validate
                Ok(FetchManyResponseKind::Found(self.records.clone()))
            }
        }
    }

    struct MockGuestUserOnAccountUpdating {
        updated_record: Option<GuestUserOnAccount>,
        should_fail: bool,
    }

    impl MockGuestUserOnAccountUpdating {
        fn new() -> Self {
            Self {
                updated_record: None,
                should_fail: false,
            }
        }

        fn with_error() -> Self {
            Self {
                updated_record: None,
                should_fail: true,
            }
        }
    }

    #[async_trait]
    impl GuestUserOnAccountUpdating for MockGuestUserOnAccountUpdating {
        async fn accept_invitation(
            &self,
            _: String,
            _: Uuid,
            _: Permission,
        ) -> Result<
            UpdatingResponseKind<(String, Uuid, Permission)>,
            MappedErrors,
        > {
            unimplemented!()
        }

        async fn update(
            &self,
            record: GuestUserOnAccount,
        ) -> Result<UpdatingResponseKind<GuestUserOnAccount>, MappedErrors>
        {
            if self.should_fail {
                return use_case_err("Update failed".to_string()).as_error();
            }

            Ok(UpdatingResponseKind::Updated(
                self.updated_record.clone().unwrap_or(record),
            ))
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Test cases
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn test_update_flags_success() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let guest_user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let guest_user_on_account = create_guest_user_on_account(
            guest_user_id,
            account_id,
            vec!["old_permit_flag".to_string()],
            vec!["old_deny_flag".to_string()],
        );

        let new_permit_flags = vec![
            "new_permit_flag1".to_string(),
            "new_permit_flag2".to_string(),
        ];
        let new_deny_flags = vec!["new_deny_flag1".to_string()];

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching =
            MockGuestUserOnAccountFetching::with_single_record(
                guest_user_on_account.clone(),
            );
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            new_permit_flags.clone(),
            new_deny_flags.clone(),
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_ok());

        let updated = match result.unwrap() {
            UpdatingResponseKind::Updated(record) => record,
            _ => panic!("Expected Updated response"),
        };

        assert_eq!(updated.permit_flags, new_permit_flags);
        assert_eq!(updated.deny_flags, new_deny_flags);
        assert_eq!(updated.guest_user_id, guest_user_id);
        assert_eq!(updated.account_id, account_id);
    }

    #[tokio::test]
    async fn test_update_flags_account_not_found() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account_fetching = MockAccountFetching::not_found();
        let guest_user_fetching = MockGuestUserOnAccountFetching::not_found();
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("Target account not found"));
        assert!(error.has_str_code(NativeErrorCodes::MYC00013.as_str()));
    }

    #[tokio::test]
    async fn test_update_flags_account_not_subscription() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let mut account = Account::default();
        account.id = Some(account_id);
        account.account_type = AccountType::User;
        account.verbose_status = Some(VerboseStatus::Verified);

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching = MockGuestUserOnAccountFetching::not_found();
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("Only subscription accounts"));
    }

    #[tokio::test]
    async fn test_update_flags_account_not_verified() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Unverified),
        );

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching = MockGuestUserOnAccountFetching::not_found();
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("Only active accounts"));
    }

    #[tokio::test]
    async fn test_update_flags_account_no_status() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(account_id, tenant_id, None);

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching = MockGuestUserOnAccountFetching::not_found();
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("Unable to check account status"));
    }

    #[tokio::test]
    async fn test_update_flags_no_guest_user_found() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching = MockGuestUserOnAccountFetching::not_found();
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("No guest user on account found"));
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }

    #[tokio::test]
    async fn test_update_flags_multiple_guest_users() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let guest_user1 = create_guest_user_on_account(
            Uuid::new_v4(),
            account_id,
            vec![],
            vec![],
        );
        let guest_user2 = create_guest_user_on_account(
            Uuid::new_v4(),
            account_id,
            vec![],
            vec![],
        );

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching =
            MockGuestUserOnAccountFetching::with_multiple_records(vec![
                guest_user1,
                guest_user2,
            ]);
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .msg()
            .contains("Operation restricted to single guest user"));
        assert!(error.has_str_code(NativeErrorCodes::MYC00018.as_str()));
    }

    #[tokio::test]
    async fn test_update_flags_paginated_response_error() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let guest_user = create_guest_user_on_account(
            Uuid::new_v4(),
            account_id,
            vec![],
            vec![],
        );

        // Simular resposta paginada (que não deveria acontecer)
        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching =
            MockGuestUserOnAccountFetching::with_paginated_response(vec![
                guest_user.clone(),
                guest_user,
            ]);
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error
            .msg()
            .contains("Internal error on update permit and deny flags"));
    }

    #[tokio::test]
    async fn test_update_flags_empty_flags() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let guest_user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let guest_user_on_account = create_guest_user_on_account(
            guest_user_id,
            account_id,
            vec!["old_flag".to_string()],
            vec!["old_deny".to_string()],
        );

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching =
            MockGuestUserOnAccountFetching::with_single_record(
                guest_user_on_account.clone(),
            );
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_ok());

        let updated = match result.unwrap() {
            UpdatingResponseKind::Updated(record) => record,
            _ => panic!("Expected Updated response"),
        };

        assert!(updated.permit_flags.is_empty());
        assert!(updated.deny_flags.is_empty());
    }

    #[tokio::test]
    async fn test_update_flags_with_tenant_manager_role() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let guest_user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        // For TenantManager, we need to give access to the target account_id
        // The use case checks for SubscriptionsManager role on the account
        // So we need to add a licensed resource with SubscriptionsManager role
        use crate::domain::actors::SystemActor;
        use crate::domain::dtos::profile::{TenantOwnership, TenantsOwnership};
        use chrono::Local;

        let mut profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "TenantManager",
            Permission::Write,
        );

        // Add a licensed resource with SubscriptionsManager role for the target account
        if let Some(LicensedResources::Records(mut resources)) =
            profile.licensed_resources
        {
            resources.push(LicensedResource {
                acc_id: account_id, // Target account ID
                tenant_id,
                role_id: Uuid::new_v4(),
                acc_name: "Target Account".to_string(),
                sys_acc: true,
                role: SystemActor::SubscriptionsManager.str().to_string(),
                perm: Permission::Write,
                verified: true,
                permit_flags: None,
                deny_flags: None,
            });

            profile = Profile::new(
                profile.owners,
                profile.acc_id,
                profile.is_subscription,
                profile.is_manager,
                profile.is_staff,
                profile.owner_is_active,
                profile.account_is_active,
                profile.account_was_approved,
                profile.account_was_archived,
                profile.account_was_deleted,
                profile.verbose_status,
                Some(LicensedResources::Records(resources)),
                Some(TenantsOwnership::Records(vec![TenantOwnership {
                    id: tenant_id,
                    name: "Test Tenant".to_string(),
                    since: Local::now(),
                }])),
            );
        }

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let guest_user_on_account = create_guest_user_on_account(
            guest_user_id,
            account_id,
            vec![],
            vec![],
        );

        let new_permit_flags = vec!["flag1".to_string(), "flag2".to_string()];
        let new_deny_flags = vec!["deny1".to_string()];

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching =
            MockGuestUserOnAccountFetching::with_single_record(
                guest_user_on_account,
            );
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            new_permit_flags.clone(),
            new_deny_flags.clone(),
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_ok());

        let updated = match result.unwrap() {
            UpdatingResponseKind::Updated(record) => record,
            _ => panic!("Expected Updated response"),
        };

        assert_eq!(updated.permit_flags, new_permit_flags);
        assert_eq!(updated.deny_flags, new_deny_flags);
    }

    #[tokio::test]
    async fn test_update_flags_account_id_none() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let mut account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );
        account.id = None; // Simular account sem ID

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching = MockGuestUserOnAccountFetching::not_found();
        let guest_user_updating = MockGuestUserOnAccountUpdating::new();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec![],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("Unable to find account id"));
    }

    #[tokio::test]
    async fn test_update_flags_update_repository_error() {
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let guest_role_id = Uuid::new_v4();
        let guest_user_id = Uuid::new_v4();
        let role_id = Uuid::new_v4();

        let profile = create_profile_with_permissions(
            tenant_id,
            account_id,
            role_id,
            "SubscriptionsManager",
            Permission::Write,
        );

        let account = create_subscription_account(
            account_id,
            tenant_id,
            Some(VerboseStatus::Verified),
        );

        let guest_user_on_account = create_guest_user_on_account(
            guest_user_id,
            account_id,
            vec![],
            vec![],
        );

        let account_fetching = MockAccountFetching::with_account(account);
        let guest_user_fetching =
            MockGuestUserOnAccountFetching::with_single_record(
                guest_user_on_account,
            );
        let guest_user_updating = MockGuestUserOnAccountUpdating::with_error();

        let account_fetching_repo =
            Box::new(&account_fetching as &dyn AccountFetching);
        let guest_user_fetching_repo =
            Box::new(&guest_user_fetching as &dyn GuestUserOnAccountFetching);
        let guest_user_updating_repo =
            Box::new(&guest_user_updating as &dyn GuestUserOnAccountUpdating);

        let result = update_flags_from_subscription_account(
            profile,
            tenant_id,
            guest_role_id,
            account_id,
            vec!["flag1".to_string()],
            vec![],
            account_fetching_repo,
            guest_user_fetching_repo,
            guest_user_updating_repo,
        )
        .await;

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.msg().contains("Update failed"));
    }
}
