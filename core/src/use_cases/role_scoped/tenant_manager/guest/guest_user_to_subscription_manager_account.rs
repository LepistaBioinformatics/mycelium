use crate::{
    domain::{
        dtos::{
            account::VerboseStatus, account_type::AccountType, email::Email,
            guest_role::Permission, guest_user::GuestUser,
            native_error_codes::NativeErrorCodes, profile::Profile,
        },
        entities::{
            AccountFetching, GuestUserRegistration, LocalMessageWrite,
            TenantFetching,
        },
    },
    models::AccountLifeCycle,
    settings::DEFAULT_TENANT_ID_KEY,
    use_cases::support::dispatch_notification,
};

use mycelium_base::{
    dtos::Parent,
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "Guest user to subscription manager account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
        guest_email = %email.redacted_email(),
    ),
    skip(
        profile,
        email,
        life_cycle_settings,
        account_fetching_repo,
        guest_user_registration_repo,
        message_sending_repo,
        tenant_fetching_repo,
    )
)]
pub async fn guest_user_to_subscription_manager_account(
    profile: Profile,
    email: Email,
    tenant_id: Uuid,
    permission: Permission,
    account_id: Uuid,
    life_cycle_settings: AccountLifeCycle,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    let span: tracing::Span = tracing::Span::current();

    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .get_tenant_wide_permission_or_error(tenant_id, Permission::Write)?;

    // ? -----------------------------------------------------------------------
    // ? Fetch account from data store
    // ? -----------------------------------------------------------------------

    let account = match account_fetching_repo
        .get(account_id, related_accounts)
        .instrument(span.clone())
        .await?
    {
        FetchResponseKind::Found(account) => account,
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!("Account not found: {}", id.unwrap()))
                .with_code(NativeErrorCodes::MYC00018)
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Check if the account is a subscription manager account
    // ? -----------------------------------------------------------------------

    let (read_role_id, write_role_id, role_name) =
        if let AccountType::RoleAssociated {
            tenant_id: account_tenant_id,
            read_role_id,
            write_role_id,
            role_name,
        } = account.account_type
        {
            if account_tenant_id != tenant_id {
                return use_case_err(format!(
                    "Account is not a subscription manager account: {}",
                    account_id
                ))
                .with_code(NativeErrorCodes::MYC00018)
                .as_error();
            }

            (read_role_id, write_role_id, role_name)
        } else {
            return use_case_err(format!(
                "Account is not a subscription manager account: {}",
                account_id
            ))
            .with_code(NativeErrorCodes::MYC00018)
            .as_error();
        };

    // ? -----------------------------------------------------------------------
    // ? Check if the account is active
    // ? -----------------------------------------------------------------------

    if let Some(status) = account.verbose_status {
        if status != VerboseStatus::Verified {
            return use_case_err(
                "Invalid account status. Only active accounts should receive \
                guesting.",
            )
            .as_error();
        }
    } else {
        return use_case_err(
            "Unable to check account status for guesting. Account is maybe \
            inactive.",
        )
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Create guest user
    // ? -----------------------------------------------------------------------

    let guest_role = match permission {
        Permission::Read => Parent::Id(read_role_id),
        Permission::Write => Parent::Id(write_role_id),
    };

    let guest_user = match guest_user_registration_repo
        .get_or_create(
            GuestUser::new_unverified(email.to_owned(), guest_role, None),
            account_id,
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
            ("account_name", account.name.to_uppercase()),
            ("role_name", role_name.to_uppercase()),
            ("role_permissions", permission.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            dtos::{
                account::Account, related_accounts::RelatedAccounts,
                telegram::TelegramUserId,
            },
            entities::{MockLocalMessageWrite, MockTenantFetching},
        },
        models::{HmacSecretEntry, HmacSecretSet},
    };

    use async_trait::async_trait;
    use myc_config::secret_resolver::SecretResolver;
    use mycelium_base::entities::FetchManyResponseKind;

    fn config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("example.com".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: None,
            noreply_email: SecretResolver::Value(
                "noreply@example.com".to_string(),
            ),
            support_name: None,
            support_email: SecretResolver::Value(
                "support@example.com".to_string(),
            ),
            token_secret: SecretResolver::Value("placeholder".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("placeholder".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    fn manager_account_on(tenant_id: Uuid) -> Account {
        let mut account = Account::default();
        account.account_type = AccountType::RoleAssociated {
            tenant_id,
            role_name: "subscriptions-manager".to_string(),
            read_role_id: Uuid::new_v4(),
            write_role_id: Uuid::new_v4(),
        };
        account
    }

    struct StubAccountFetching(Account);

    #[async_trait]
    impl AccountFetching for StubAccountFetching {
        async fn get(
            &self,
            _: Uuid,
            _: RelatedAccounts,
        ) -> Result<FetchResponseKind<Account, Uuid>, MappedErrors> {
            Ok(FetchResponseKind::Found(self.0.to_owned()))
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

    struct UnreachableGuestUserRegistration;

    #[async_trait]
    impl GuestUserRegistration for UnreachableGuestUserRegistration {
        async fn get_or_create(
            &self,
            _: GuestUser,
            _: Uuid,
        ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
            unreachable!("guesting must stop before registration")
        }
    }

    async fn guest_onto(
        requested_tenant: Uuid,
        account: Account,
    ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
        let mut profile = Profile::default();
        profile.is_staff = true;

        guest_user_to_subscription_manager_account(
            profile,
            Email::from_string("user-01@example.com".to_string()).unwrap(),
            requested_tenant,
            Permission::Read,
            Uuid::new_v4(),
            config(),
            Box::new(&StubAccountFetching(account)),
            Box::new(&UnreachableGuestUserRegistration),
            Box::new(&MockLocalMessageWrite::new()),
            Box::new(&MockTenantFetching::new()),
        )
        .await
    }

    #[tokio::test]
    async fn refuses_a_manager_account_of_another_tenant() {
        let account = manager_account_on(Uuid::new_v4());

        let err = guest_onto(Uuid::new_v4(), account).await.unwrap_err();

        assert!(err.has_str_code("MYC00018"), "{err}");
    }

    #[tokio::test]
    async fn accepts_the_tenant_of_the_manager_account() {
        let tenant_id = Uuid::new_v4();

        let err = guest_onto(tenant_id, manager_account_on(tenant_id))
            .await
            .unwrap_err();

        assert!(!err.has_str_code("MYC00018"), "{err}");
        assert!(err.to_string().contains("account status"), "{err}");
    }
}
