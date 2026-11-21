use crate::models::AccountLifeCycle;

use mycelium_base::utils::errors::MappedErrors;

/// Locale used when nothing else informs one.
pub(crate) const FALLBACK_NOTIFICATION_LOCALE: &str = "en-us";

/// Resolve the locale of an outgoing notification
///
/// The first one available wins, in this order:
///
/// 1. the locale asked for by the request;
/// 2. the tenant preferred locale (tenant meta);
/// 3. the instance locale (`AccountLifeCycle.locale`);
/// 4. `en-us`.
///
/// The result is only a preference: the caller still has to check that a
/// template exists for it.
#[tracing::instrument(name = "resolve_notification_locale", skip_all)]
pub(crate) async fn resolve_notification_locale(
    requested_locale: Option<String>,
    tenant_locale: Option<String>,
    config: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    if let Some(locale) = requested_locale {
        tracing::trace!("Communicating with requested locale: {:?}", locale);
        return Ok(locale);
    }

    if let Some(locale) = tenant_locale {
        tracing::trace!("Communicating with tenant locale: {:?}", locale);
        return Ok(locale);
    }

    if let Some(locale) = &config.locale {
        tracing::trace!("Communicating with system locale: {:?}", locale);
        return locale.async_get_or_error().await;
    }

    tracing::trace!("Communicating with default locale: 'en-us'");
    Ok(FALLBACK_NOTIFICATION_LOCALE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HmacSecretEntry, HmacSecretSet};
    use myc_config::secret_resolver::SecretResolver;

    fn config(locale: Option<&str>) -> AccountLifeCycle {
        let secret = |value: &str| SecretResolver::Value(value.to_string());

        AccountLifeCycle {
            domain_name: secret("Test Domain"),
            domain_url: None,
            locale: locale.map(secret),
            token_expiration: SecretResolver::Value(3600),
            noreply_name: None,
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

    fn some(locale: &str) -> Option<String> {
        Some(locale.to_string())
    }

    #[tokio::test]
    async fn requested_locale_wins() -> Result<(), MappedErrors> {
        let locale = resolve_notification_locale(
            some("es"),
            some("pt-br"),
            &config(Some("en-us")),
        )
        .await?;

        assert_eq!(locale, "es");
        Ok(())
    }

    #[tokio::test]
    async fn tenant_locale_is_used_without_a_requested_one(
    ) -> Result<(), MappedErrors> {
        let locale = resolve_notification_locale(
            None,
            some("pt-br"),
            &config(Some("en-us")),
        )
        .await?;

        assert_eq!(locale, "pt-br");
        Ok(())
    }

    #[tokio::test]
    async fn config_locale_is_used_without_request_and_tenant(
    ) -> Result<(), MappedErrors> {
        let locale =
            resolve_notification_locale(None, None, &config(Some("es")))
                .await?;

        assert_eq!(locale, "es");
        Ok(())
    }

    #[tokio::test]
    async fn falls_back_to_en_us_when_nothing_is_informed(
    ) -> Result<(), MappedErrors> {
        let locale =
            resolve_notification_locale(None, None, &config(None)).await?;

        assert_eq!(locale, "en-us");
        Ok(())
    }
}
