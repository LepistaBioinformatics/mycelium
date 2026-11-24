use crate::{
    domain::dtos::{
        email::Email,
        message::FromEmail,
        service_notification_config::{
            validate_sender_tag, ServiceNotificationConfig,
        },
    },
    models::{AccountLifeCycle, SenderTagMode},
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use uuid::Uuid;

/// Mail header naming the service a message was sent on behalf of.
pub const SOURCE_SERVICE_HEADER: &str = "X-Mycelium-Source-Service";

/// Mail header carrying the dispatch id of a service message.
pub const MESSAGE_ID_HEADER: &str = "X-Mycelium-Message-Id";

/// Build the `From` and the attribution headers of a service email
///
/// - display name: `"{displayName} via {noreplyName}"`, or `displayName`
///   alone when the instance has no `noreplyName`;
/// - address in `header` mode: the instance `noreplyEmail`, untouched;
/// - address in `plus` mode: `local+senderTag@domain`;
/// - headers, in both modes: `X-Mycelium-Source-Service` and
///   `X-Mycelium-Message-Id`.
///
/// The sender tag is checked again against the `senderTag` charset
/// (`^[a-z0-9-]{1,32}$`) before it reaches an address or a header, so it
/// can not carry `@`, whitespace or a line break even if the config struct
/// was not built through its validated deserialization.
#[tracing::instrument(name = "build_service_sender", skip_all)]
pub async fn build_service_sender(
    mode: SenderTagMode,
    life_cycle: &AccountLifeCycle,
    config: &ServiceNotificationConfig,
    message_id: Uuid,
) -> Result<(FromEmail, Vec<(String, String)>), MappedErrors> {
    validate_sender_tag(&config.sender_tag).map_err(use_case_err)?;

    let display_name = build_display_name(life_cycle, config).await?;
    let address = build_address(mode, life_cycle, &config.sender_tag).await?;

    let from = FromEmail::NamedEmail(format!(
        "\"{display_name}\" <{}>",
        address.email()
    ));

    let headers = vec![
        (SOURCE_SERVICE_HEADER.to_string(), config.sender_tag.clone()),
        (MESSAGE_ID_HEADER.to_string(), message_id.to_string()),
    ];

    Ok((from, headers))
}

/// Compose the display name, escaped to sit inside a quoted string.
async fn build_display_name(
    life_cycle: &AccountLifeCycle,
    config: &ServiceNotificationConfig,
) -> Result<String, MappedErrors> {
    let display_name = match &life_cycle.noreply_name {
        None => config.display_name.clone(),
        Some(noreply_name) => format!(
            "{} via {}",
            config.display_name,
            noreply_name.async_get_or_error().await?
        ),
    };

    if display_name.chars().any(char::is_control) {
        return use_case_err("Sender display name has a control character")
            .as_error();
    }

    Ok(display_name.replace('\\', "\\\\").replace('"', "\\\""))
}

async fn build_address(
    mode: SenderTagMode,
    life_cycle: &AccountLifeCycle,
    sender_tag: &str,
) -> Result<Email, MappedErrors> {
    let noreply_email = Email::from_string(
        life_cycle.noreply_email.async_get_or_error().await?,
    )?;

    match mode {
        SenderTagMode::Header => Ok(noreply_email),
        SenderTagMode::Plus => Ok(Email {
            username: format!("{}+{sender_tag}", noreply_email.username),
            domain: noreply_email.domain,
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        domain::dtos::service_notification_config::RecipientPolicy,
        models::{HmacSecretEntry, HmacSecretSet},
    };
    use myc_config::secret_resolver::SecretResolver;

    fn life_cycle(noreply_name: Option<&str>) -> AccountLifeCycle {
        let secret = |value: &str| SecretResolver::Value(value.to_string());

        AccountLifeCycle {
            domain_name: secret("Test Domain"),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(3600),
            noreply_name: noreply_name.map(secret),
            noreply_email: secret("noreply@example.com"),
            support_name: None,
            support_email: secret("support@example.com"),
            token_secret: secret("test-secret"),
            hmac_primary_version: 1,
            hmac_secrets: HmacSecretSet::new(vec![HmacSecretEntry {
                version: 1,
                secret: secret("test-hmac"),
            }]),
            staff_bootstrap_secret: None,
        }
    }

    fn service(
        sender_tag: &str,
        display_name: &str,
    ) -> ServiceNotificationConfig {
        ServiceNotificationConfig {
            sender_tag: sender_tag.to_string(),
            display_name: display_name.to_string(),
            issuer_roles: vec!["service-01-admin".to_string()],
            issuer_tenants: vec![],
            recipient_policy: RecipientPolicy::Owner,
            allowed_domains: vec![],
            templates: vec!["invoice-ready".to_string()],
            quota_per_hour: 100,
            max_recipients_per_message: 10,
        }
    }

    fn named(from: FromEmail) -> String {
        match from {
            FromEmail::NamedEmail(named) => named,
            FromEmail::Email(email) => panic!("unexpected bare {email:?}"),
        }
    }

    fn expected_headers(message_id: Uuid) -> Vec<(String, String)> {
        vec![
            (
                "X-Mycelium-Source-Service".to_string(),
                "service-01".to_string(),
            ),
            ("X-Mycelium-Message-Id".to_string(), message_id.to_string()),
        ]
    }

    #[tokio::test]
    async fn header_mode_keeps_the_noreply_address() -> Result<(), MappedErrors>
    {
        let message_id = Uuid::new_v4();

        let (from, headers) = build_service_sender(
            SenderTagMode::Header,
            &life_cycle(Some("Mycelium")),
            &service("service-01", "Service 01"),
            message_id,
        )
        .await?;

        assert_eq!(
            named(from),
            "\"Service 01 via Mycelium\" <noreply@example.com>"
        );
        assert_eq!(headers, expected_headers(message_id));
        Ok(())
    }

    #[tokio::test]
    async fn plus_mode_appends_the_tag_to_the_local_part(
    ) -> Result<(), MappedErrors> {
        let message_id = Uuid::new_v4();

        let (from, headers) = build_service_sender(
            SenderTagMode::Plus,
            &life_cycle(Some("Mycelium")),
            &service("service-01", "Service 01"),
            message_id,
        )
        .await?;

        assert_eq!(
            named(from),
            "\"Service 01 via Mycelium\" <noreply+service-01@example.com>"
        );
        assert_eq!(headers, expected_headers(message_id));
        Ok(())
    }

    #[tokio::test]
    async fn absent_noreply_name_leaves_the_service_name_alone(
    ) -> Result<(), MappedErrors> {
        for mode in [SenderTagMode::Header, SenderTagMode::Plus] {
            let (from, headers) = build_service_sender(
                mode,
                &life_cycle(None),
                &service("service-01", "Service 01"),
                Uuid::new_v4(),
            )
            .await?;

            assert!(named(from).starts_with("\"Service 01\" <noreply"));
            assert_eq!(headers.len(), 2);
        }

        Ok(())
    }

    #[tokio::test]
    async fn tag_outside_the_sender_tag_charset_is_rejected_in_both_modes() {
        let tags = [
            "evil@attacker.example",
            "service 01",
            "service-01\r\nBcc: attacker@example.com",
            "service-01\n",
            "a+b",
            "a>,<b@c.example",
            "Service-01",
            "",
        ];

        for mode in [SenderTagMode::Header, SenderTagMode::Plus] {
            for tag in tags {
                let result = build_service_sender(
                    mode,
                    &life_cycle(Some("Mycelium")),
                    &service(tag, "Service 01"),
                    Uuid::new_v4(),
                )
                .await;

                assert!(result.is_err(), "{mode:?} {tag:?}");
            }
        }
    }

    #[tokio::test]
    async fn longest_valid_tag_is_accepted_in_plus_mode(
    ) -> Result<(), MappedErrors> {
        let tag = "a".repeat(32);

        let (from, _) = build_service_sender(
            SenderTagMode::Plus,
            &life_cycle(None),
            &service(&tag, "Service 01"),
            Uuid::new_v4(),
        )
        .await?;

        assert!(named(from).ends_with(&format!("<noreply+{tag}@example.com>")));
        Ok(())
    }

    #[tokio::test]
    async fn quotes_in_the_display_name_can_not_close_the_quoted_string(
    ) -> Result<(), MappedErrors> {
        let (from, _) = build_service_sender(
            SenderTagMode::Header,
            &life_cycle(Some("My\\celium")),
            &service("service-01", "A\" <attacker@example.com>"),
            Uuid::new_v4(),
        )
        .await?;

        assert_eq!(
            named(from),
            "\"A\\\" <attacker@example.com> via My\\\\celium\" \
             <noreply@example.com>"
        );
        Ok(())
    }

    #[tokio::test]
    async fn line_break_in_the_display_name_is_rejected() {
        let result = build_service_sender(
            SenderTagMode::Header,
            &life_cycle(Some("Mycelium\r\nBcc: attacker@example.com")),
            &service("service-01", "Service 01"),
            Uuid::new_v4(),
        )
        .await;

        assert!(result.is_err());
    }
}
