use crate::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        profile::Profile,
        service_notification_config::{
            RecipientPolicy, ServiceNotificationConfig,
        },
    },
    entities::LicensedResourcesFetching,
};

use futures::future::try_join_all;
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use serde::Serialize;
use uuid::Uuid;

/// Check if a service may send a notification to every recipient
///
/// - `owner`: every recipient is the principal owner of the issuer account.
/// - `domains`: every recipient domain is listed in `allowedDomains`. The
///   whole domain must match, ignoring case: a listed domain allows neither
///   its subdomains nor domains that merely contain it.
/// - `tenantMembers`: every recipient is a guest or an owner of the
///   connection string tenant.
/// - `any`: no restriction.
///
/// Every violation returns the same MYC00039, which never names the failing
/// address: a service must not learn who is a member of a tenant from it.
#[tracing::instrument(name = "check_recipient_policy", skip_all)]
pub async fn check_recipient_policy(
    config: &ServiceNotificationConfig,
    profile: &Profile,
    tenant_id: Option<Uuid>,
    recipients: &[Email],
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<(), MappedErrors> {
    let allowed = match config.recipient_policy {
        RecipientPolicy::Any => true,
        RecipientPolicy::Owner => {
            every_recipient_is_the_principal_owner(profile, recipients)
        }
        RecipientPolicy::Domains => every_recipient_domain_is_allowed(
            &config.allowed_domains,
            recipients,
        ),
        RecipientPolicy::TenantMembers => {
            every_recipient_is_a_tenant_member(
                tenant_id,
                recipients,
                *licensed_resources_fetching_repo,
            )
            .await?
        }
    };

    if !allowed {
        return recipient_not_allowed();
    }

    Ok(())
}

fn every_recipient_is_the_principal_owner(
    profile: &Profile,
    recipients: &[Email],
) -> bool {
    let Some(owner) = profile.owners.iter().find(|owner| owner.is_principal)
    else {
        return false;
    };

    let owner_email = owner.email.to_lowercase();

    recipients
        .iter()
        .all(|recipient| recipient.email() == owner_email)
}

fn every_recipient_domain_is_allowed(
    allowed_domains: &[String],
    recipients: &[Email],
) -> bool {
    let allowed_domains: Vec<String> = allowed_domains
        .iter()
        .map(|domain| domain.to_lowercase())
        .collect();

    recipients.iter().all(|recipient| {
        allowed_domains.contains(&recipient.domain.to_lowercase())
    })
}

async fn every_recipient_is_a_tenant_member(
    tenant_id: Option<Uuid>,
    recipients: &[Email],
    licensed_resources_fetching_repo: &dyn LicensedResourcesFetching,
) -> Result<bool, MappedErrors> {
    let Some(tenant_id) = tenant_id else {
        return Ok(false);
    };

    let memberships = try_join_all(recipients.iter().map(|recipient| {
        is_a_tenant_member(
            recipient,
            tenant_id,
            licensed_resources_fetching_repo,
        )
    }))
    .await?;

    Ok(memberships.into_iter().all(|is_member| is_member))
}

/// Whether the address is licensed on, or owns, the tenant
async fn is_a_tenant_member(
    recipient: &Email,
    tenant_id: Uuid,
    licensed_resources_fetching_repo: &dyn LicensedResourcesFetching,
) -> Result<bool, MappedErrors> {
    let licensed_resources = licensed_resources_fetching_repo
        .list_licensed_resources(
            recipient.to_owned(),
            Some(tenant_id),
            None,
            None,
            None,
        )
        .await?;

    if has_records(&licensed_resources) {
        return Ok(true);
    }

    let ownerships = licensed_resources_fetching_repo
        .list_tenants_ownership(recipient.to_owned(), Some(tenant_id))
        .await?;

    Ok(has_records(&ownerships))
}

fn has_records<T: Serialize>(response: &FetchManyResponseKind<T>) -> bool {
    match response {
        FetchManyResponseKind::NotFound => false,
        FetchManyResponseKind::Found(records) => !records.is_empty(),
        FetchManyResponseKind::FoundPaginated { records, .. } => {
            !records.is_empty()
        }
    }
}

fn recipient_not_allowed() -> Result<(), MappedErrors> {
    use_case_err("Recipient not allowed by the service recipient policy")
        .with_code(NativeErrorCodes::MYC00039)
        .with_exp_true()
        .as_error()
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            guest_role::Permission,
            profile::{LicensedResource, Owner, TenantOwnership},
        },
        entities::MockLicensedResourcesFetching,
    };

    use chrono::Local;

    const OWNER_EMAIL: &str = "owner@example.com";

    fn config(
        policy: &str,
        allowed_domains: &[&str],
    ) -> ServiceNotificationConfig {
        serde_json::from_value(serde_json::json!({
            "senderTag": "service-01",
            "displayName": "Service 01",
            "issuerRoles": ["service-01-admin"],
            "recipientPolicy": policy,
            "allowedDomains": allowed_domains,
            "templates": ["welcome"],
        }))
        .unwrap()
    }

    fn owner(email: &str, is_principal: bool) -> Owner {
        Owner {
            id: Uuid::new_v4(),
            email: email.to_string(),
            first_name: None,
            last_name: None,
            username: None,
            is_principal,
        }
    }

    fn profile(owners: Vec<Owner>) -> Profile {
        Profile::new(
            owners,
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
        )
    }

    fn principal_profile() -> Profile {
        profile(vec![
            owner("secondary@example.com", false),
            owner(OWNER_EMAIL, true),
        ])
    }

    fn emails(addresses: &[&str]) -> Vec<Email> {
        addresses
            .iter()
            .map(|address| Email::from_string(address.to_string()).unwrap())
            .collect()
    }

    fn licensed_resource(tenant_id: Uuid) -> LicensedResource {
        LicensedResource {
            acc_id: Uuid::new_v4(),
            tenant_id,
            role_id: Uuid::new_v4(),
            acc_name: "Account".to_string(),
            sys_acc: false,
            role: "member".to_string(),
            perm: Permission::Read,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }
    }

    /// A repository where `guests` are licensed on the tenant and `owners`
    /// own it. Asking about any other tenant fails the test.
    fn tenant_repo(
        tenant_id: Uuid,
        guests: &'static [&'static str],
        owners: &'static [&'static str],
    ) -> MockLicensedResourcesFetching {
        let mut repo = MockLicensedResourcesFetching::new();

        repo.expect_list_licensed_resources().returning(
            move |email, tenant, _, _, _| {
                assert_eq!(tenant, Some(tenant_id));

                match guests.contains(&email.email().as_str()) {
                    true => Ok(FetchManyResponseKind::Found(vec![
                        licensed_resource(tenant_id),
                    ])),
                    false => Ok(FetchManyResponseKind::NotFound),
                }
            },
        );

        repo.expect_list_tenants_ownership()
            .returning(move |email, tenant| {
                assert_eq!(tenant, Some(tenant_id));

                match owners.contains(&email.email().as_str()) {
                    true => Ok(FetchManyResponseKind::Found(vec![
                        TenantOwnership {
                            id: tenant_id,
                            name: "Tenant 01".to_string(),
                            since: Local::now(),
                        },
                    ])),
                    false => Ok(FetchManyResponseKind::Found(vec![])),
                }
            });

        repo
    }

    /// A repository that fails the test when it is asked anything.
    fn untouched_repo() -> MockLicensedResourcesFetching {
        MockLicensedResourcesFetching::new()
    }

    async fn check(
        config: &ServiceNotificationConfig,
        tenant_id: Option<Uuid>,
        recipients: &[&str],
        repo: &MockLicensedResourcesFetching,
    ) -> Result<(), MappedErrors> {
        check_recipient_policy(
            config,
            &principal_profile(),
            tenant_id,
            &emails(recipients),
            Box::new(repo),
        )
        .await
    }

    fn assert_not_allowed(result: Result<(), MappedErrors>) {
        let error = result.unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00039]), "{error}");
        assert!(error.expected());
    }

    // ? -----------------------------------------------------------------------
    // ? owner
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn owner_policy_accepts_the_principal_owner(
    ) -> Result<(), MappedErrors> {
        let config = config("owner", &[]);

        check(&config, None, &[OWNER_EMAIL], &untouched_repo()).await?;
        check(&config, None, &["Owner@EXAMPLE.com"], &untouched_repo()).await
    }

    #[tokio::test]
    async fn owner_policy_rejects_any_other_address() {
        let config = config("owner", &[]);

        for recipients in [
            vec!["other@example.com"],
            vec!["secondary@example.com"],
            vec![OWNER_EMAIL, "other@example.com"],
        ] {
            assert_not_allowed(
                check(&config, None, &recipients, &untouched_repo()).await,
            );
        }
    }

    #[tokio::test]
    async fn owner_policy_rejects_when_there_is_no_principal_owner() {
        let result = check_recipient_policy(
            &config("owner", &[]),
            &profile(vec![owner(OWNER_EMAIL, false)]),
            None,
            &emails(&[OWNER_EMAIL]),
            Box::new(&untouched_repo()),
        )
        .await;

        assert_not_allowed(result);
    }

    // ? -----------------------------------------------------------------------
    // ? domains
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn domains_policy_accepts_listed_domains_ignoring_case(
    ) -> Result<(), MappedErrors> {
        let config = config("domains", &["Allowed.Example", "other.example"]);

        check(
            &config,
            None,
            &[
                "user@allowed.example",
                "user@ALLOWED.example",
                "user@other.example",
            ],
            &untouched_repo(),
        )
        .await
    }

    #[tokio::test]
    async fn domains_policy_rejects_anything_but_the_exact_domain() {
        let config = config("domains", &["allowed.example"]);

        for recipient in [
            "user@allowed.example.evil.example",
            "user@evil.example",
            "user@notallowed.example",
            "user@sub.allowed.example",
            "user@allowed.examples",
            "allowed.example@evil.example",
        ] {
            let result =
                check(&config, None, &[recipient], &untouched_repo()).await;

            assert!(result.is_err(), "{recipient} must be rejected");
            assert_not_allowed(result);
        }
    }

    #[tokio::test]
    async fn domains_policy_rejects_a_batch_with_one_foreign_domain() {
        let config = config("domains", &["allowed.example"]);

        assert_not_allowed(
            check(
                &config,
                None,
                &["user@allowed.example", "user@evil.example"],
                &untouched_repo(),
            )
            .await,
        );
    }

    // ? -----------------------------------------------------------------------
    // ? tenantMembers
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn tenant_members_policy_accepts_guests_and_owners(
    ) -> Result<(), MappedErrors> {
        let tenant_id = Uuid::new_v4();
        let repo = tenant_repo(
            tenant_id,
            &["guest@example.com"],
            &["tenant-owner@example.com"],
        );

        check(
            &config("tenantMembers", &[]),
            Some(tenant_id),
            &["guest@example.com", "tenant-owner@example.com"],
            &repo,
        )
        .await
    }

    #[tokio::test]
    async fn tenant_members_policy_rejects_a_non_member() {
        let tenant_id = Uuid::new_v4();
        let repo = tenant_repo(tenant_id, &["guest@example.com"], &[]);

        assert_not_allowed(
            check(
                &config("tenantMembers", &[]),
                Some(tenant_id),
                &["guest@example.com", "stranger@example.com"],
                &repo,
            )
            .await,
        );
    }

    #[tokio::test]
    async fn tenant_members_policy_rejects_without_a_tenant() {
        assert_not_allowed(
            check(
                &config("tenantMembers", &[]),
                None,
                &["guest@example.com"],
                &untouched_repo(),
            )
            .await,
        );
    }

    #[tokio::test]
    async fn violation_never_names_the_failing_address() {
        let tenant_id = Uuid::new_v4();
        let repo = tenant_repo(tenant_id, &["guest@example.com"], &[]);
        let cases = [
            (config("tenantMembers", &[]), Some(tenant_id)),
            (config("domains", &["allowed.example"]), None),
            (config("owner", &[]), None),
        ];

        for (config, tenant_id) in cases {
            let error = check(
                &config,
                tenant_id,
                &["guest@example.com", "stranger@elsewhere.example"],
                &repo,
            )
            .await
            .unwrap_err();

            let rendered = format!("{error} {error:?}").to_lowercase();

            for leaked in ["stranger", "elsewhere.example", "guest@"] {
                assert!(!rendered.contains(leaked), "{rendered}");
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ? any
    // ? -----------------------------------------------------------------------

    #[tokio::test]
    async fn any_policy_accepts_every_address() -> Result<(), MappedErrors> {
        check(
            &config("any", &[]),
            None,
            &["anyone@anywhere.example"],
            &untouched_repo(),
        )
        .await
    }
}
