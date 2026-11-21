use crate::{
    domain::{
        dtos::{
            email::Email, native_error_codes::NativeErrorCodes,
            token::UserAccountConnectionString,
        },
        entities::{LocalMessageWrite, TenantFetching, TokenRegistration},
    },
    models::AccountLifeCycle,
    settings::DEFAULT_TENANT_ID_KEY,
    use_cases::support::dispatch_notification,
};

use chrono::{DateTime, Duration, Local};
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Register a signed connection string and email its owner
///
/// Shared tail of every connection string issuance: store the token, then
/// tell the principal owner a new string exists. Returns the string to hand
/// to the caller.
#[tracing::instrument(name = "register_and_notify_connection_string", skip_all)]
pub(crate) async fn register_and_notify_connection_string(
    connection_string: UserAccountConnectionString,
    expires_at: DateTime<Local>,
    expiration: i64,
    tenant_id: Option<Uuid>,
    owner_email: Email,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<String, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Register the token
    // ? -----------------------------------------------------------------------

    if let CreateResponseKind::NotCreated(_, msg) = token_registration_repo
        .create_connection_string(connection_string.to_owned(), expires_at)
        .await?
    {
        tracing::error!("Unable to register connection string: {msg}");
        return use_case_err("Unable to register token").as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Notify the owner
    // ? -----------------------------------------------------------------------

    let mut parameters = vec![(
        "expires_in",
        format_expiration_as_human_readable(expiration),
    )];

    if let Some(tenant_id) = tenant_id {
        parameters.push((DEFAULT_TENANT_ID_KEY, tenant_id.to_string()));
    }

    if let Err(err) = dispatch_notification(
        parameters,
        "email/create-connection-string",
        life_cycle_settings,
        owner_email,
        None,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
    {
        tracing::error!("Unable to send email: {err}");
        return use_case_err("Unable to notify user")
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    // ? -----------------------------------------------------------------------
    // ? Send user the token
    // ? -----------------------------------------------------------------------

    Ok(connection_string.scope.to_string())
}

fn format_expiration_as_human_readable(expiration: i64) -> String {
    let duration = Duration::seconds(expiration);
    let days = duration.num_days();
    let hours = duration.num_hours() % 24;
    let minutes = duration.num_minutes() % 60;

    if days > 0 {
        return format!("{days}d");
    }

    if hours > 0 {
        return format!("{hours}h");
    }

    format!("{minutes}m")
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::{
            dtos::token::{MultiTypeMeta, Token, UserAccountScope},
            entities::{
                MockLocalMessageWrite, MockTenantFetching,
                MockTokenRegistration,
            },
        },
        models::{HmacSecretEntry, HmacSecretSet},
    };
    use myc_config::secret_resolver::SecretResolver;

    fn life_cycle() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_url: None,
            domain_name: SecretResolver::Value("Test Domain".to_string()),
            locale: None,
            token_expiration: SecretResolver::Value(30),
            noreply_name: None,
            noreply_email: SecretResolver::Value("noreply@test.com".into()),
            support_name: None,
            support_email: SecretResolver::Value("support@test.com".into()),
            token_secret: SecretResolver::Value("test-secret".to_string()),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: SecretResolver::Value("test-hmac".to_string()),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    async fn signed_connection_string(
        expires_at: DateTime<Local>,
    ) -> Result<UserAccountConnectionString, MappedErrors> {
        let account_id = Uuid::new_v4();
        let mut scope = UserAccountScope::new(
            account_id,
            expires_at,
            None,
            None,
            None,
            life_cycle(),
        )
        .await?;

        UserAccountConnectionString::new_signed_token(
            &mut scope,
            account_id,
            Email::from_string("owner@test.com".to_string())?,
            life_cycle(),
            Some("token".to_string()),
        )
        .await
    }

    #[tokio::test]
    async fn registers_the_string_then_emails_the_owner(
    ) -> Result<(), MappedErrors> {
        let expires_at = Local::now() + Duration::seconds(3600);
        let connection_string = signed_connection_string(expires_at).await?;
        let expected = connection_string.scope.to_string();

        let mut token_repo = MockTokenRegistration::new();
        token_repo
            .expect_create_connection_string()
            .times(1)
            .returning(|meta, expires| {
                Ok(CreateResponseKind::Created(Token::new(
                    None,
                    expires,
                    MultiTypeMeta::UserAccountConnectionString(meta),
                )))
            });

        let mut message_repo = MockLocalMessageWrite::new();
        message_repo
            .expect_send()
            .withf(|event| event.message.to.email() == "owner@test.com")
            .times(1)
            .returning(|_| Ok(CreateResponseKind::Created(None)));

        let tenant_repo = MockTenantFetching::new();

        let issued = register_and_notify_connection_string(
            connection_string,
            expires_at,
            3600,
            None,
            Email::from_string("owner@test.com".to_string())?,
            life_cycle(),
            Box::new(&token_repo),
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await?;

        assert_eq!(issued, expected);

        Ok(())
    }

    #[tokio::test]
    async fn unregistered_string_is_an_error_and_sends_no_email(
    ) -> Result<(), MappedErrors> {
        let expires_at = Local::now() + Duration::seconds(3600);
        let connection_string = signed_connection_string(expires_at).await?;

        let mut token_repo = MockTokenRegistration::new();
        token_repo.expect_create_connection_string().returning(
            |meta, expires| {
                Ok(CreateResponseKind::NotCreated(
                    Token::new(
                        None,
                        expires,
                        MultiTypeMeta::UserAccountConnectionString(meta),
                    ),
                    "duplicated".to_string(),
                ))
            },
        );

        let mut message_repo = MockLocalMessageWrite::new();
        message_repo.expect_send().never();

        let tenant_repo = MockTenantFetching::new();

        let result = register_and_notify_connection_string(
            connection_string,
            expires_at,
            3600,
            None,
            Email::from_string("owner@test.com".to_string())?,
            life_cycle(),
            Box::new(&token_repo),
            Box::new(&message_repo),
            Box::new(&tenant_repo),
        )
        .await;

        assert!(result.is_err());

        Ok(())
    }

    #[test]
    fn expiration_is_formatted_by_its_largest_unit() {
        assert_eq!(format_expiration_as_human_readable(2 * 86400), "2d");
        assert_eq!(format_expiration_as_human_readable(3 * 3600), "3h");
        assert_eq!(format_expiration_as_human_readable(5 * 60), "5m");
    }
}
