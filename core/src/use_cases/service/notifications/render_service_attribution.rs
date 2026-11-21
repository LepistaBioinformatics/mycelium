use crate::{
    models::AccountLifeCycle, settings::TEMPLATES,
    use_cases::support::FALLBACK_NOTIFICATION_LOCALE,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use tera::Context;

const SERVICE_ATTRIBUTION_TEMPLATE: &str = "email/service-attribution.jinja";

/// Render the footer telling the recipient which service sent the email
///
/// The footer is owned by the gateway (main `TEMPLATES`), never by the
/// service, so a service can not drop or reword its own attribution. A
/// locale without the template falls back to `en-us`. `display_name` is
/// HTML-escaped by the template autoescape.
#[tracing::instrument(name = "render_service_attribution", skip_all)]
pub async fn render_service_attribution(
    locale: &str,
    display_name: &str,
    life_cycle: &AccountLifeCycle,
) -> Result<String, MappedErrors> {
    let domain_name = life_cycle.domain_name.async_get_or_error().await?;

    let mut context = Context::new();
    context.insert("display_name", display_name);
    context.insert("domain_name", &domain_name);

    TEMPLATES
        .render(&attribution_template_for(locale), &context)
        .map_err(|err| {
            use_case_err(format!(
                "Unable to render the service attribution footer: {err}"
            ))
        })
}

fn attribution_template_for(locale: &str) -> String {
    let localized = format!("{locale}/{SERVICE_ATTRIBUTION_TEMPLATE}");

    if TEMPLATES.get_template(&localized).is_ok() {
        return localized;
    }

    format!("{FALLBACK_NOTIFICATION_LOCALE}/{SERVICE_ATTRIBUTION_TEMPLATE}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{HmacSecretEntry, HmacSecretSet};
    use myc_config::secret_resolver::SecretResolver;

    fn life_cycle() -> AccountLifeCycle {
        let secret = |value: &str| SecretResolver::Value(value.to_string());

        AccountLifeCycle {
            domain_name: secret("Test Domain"),
            domain_url: None,
            locale: None,
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

    #[tokio::test]
    async fn renders_for_every_gateway_locale() -> Result<(), MappedErrors> {
        let expectations = [
            ("en-us", "This message was sent by"),
            ("es", "Este mensaje fue enviado por"),
            ("pt-br", "Esta mensagem foi enviada por"),
        ];

        for (locale, sentence) in expectations {
            let footer =
                render_service_attribution(locale, "Service 01", &life_cycle())
                    .await?;

            assert!(footer.contains(sentence), "{locale}: {footer}");
            assert!(footer.contains("Service 01"), "{locale}: {footer}");
            assert!(footer.contains("Test Domain"), "{locale}: {footer}");
        }

        Ok(())
    }

    #[tokio::test]
    async fn display_name_is_escaped() -> Result<(), MappedErrors> {
        for locale in ["en-us", "es", "pt-br"] {
            let footer = render_service_attribution(
                locale,
                "<script>alert('x')</script> & \"co\"",
                &life_cycle(),
            )
            .await?;

            assert!(!footer.contains("<script>"), "{locale}: {footer}");
            assert!(footer.contains("&lt;script&gt;"), "{locale}: {footer}");
            assert!(footer.contains("&amp;"), "{locale}: {footer}");
            assert!(footer.contains("&quot;co&quot;"), "{locale}: {footer}");
        }

        Ok(())
    }

    #[tokio::test]
    async fn unknown_locale_falls_back_to_en_us() -> Result<(), MappedErrors> {
        for locale in ["fr-fr", "../web", ""] {
            let footer =
                render_service_attribution(locale, "Service 01", &life_cycle())
                    .await?;

            assert!(footer.contains("This message was sent by"), "{locale}");
        }

        Ok(())
    }
}
