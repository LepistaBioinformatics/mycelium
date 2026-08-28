use myc_http_tools::{
    models::external_providers_config::ExternalProviderConfig,
    responses::GatewayError, settings::MYCELIUM_PROVIDER_KEY,
};

/// Resolve the issuer that owns the authenticated identity
///
/// `check_credentials_with_multi_identity_provider` returns `None` as the
/// provider config when the token was issued by the internal (mycelium)
/// provider. A `None` is therefore not a failure: it names the internal issuer.
/// Both the REST beginners route and the `/_adm/rpc` dispatcher must resolve it
/// the same way.
///
#[tracing::instrument(name = "resolve_issuer_from_provider", skip_all)]
pub(crate) async fn resolve_issuer_from_provider(
    provider: Option<ExternalProviderConfig>,
) -> Result<String, GatewayError> {
    let Some(provider) = provider else {
        return Ok(MYCELIUM_PROVIDER_KEY.to_string());
    };

    provider.issuer.async_get_or_error().await.map_err(|err| {
        tracing::warn!("Unable to resolve the external provider issuer: {err}");

        GatewayError::InternalServerError(
            "Unable to resolve the identity provider issuer".to_string(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use myc_config::secret_resolver::SecretResolver;

    fn external_provider(issuer: &str) -> ExternalProviderConfig {
        ExternalProviderConfig {
            issuer: SecretResolver::Value(issuer.to_string()),
            jwks_uri: SecretResolver::Value(
                "https://example.com/jwks".to_string(),
            ),
            audience: SecretResolver::Value("audience".to_string()),
            discovery_url: None,
            user_info_url: None,
        }
    }

    #[tokio::test]
    async fn none_provider_resolves_to_the_internal_issuer() {
        let issuer = resolve_issuer_from_provider(None).await.unwrap();

        assert_eq!(issuer, MYCELIUM_PROVIDER_KEY);
    }

    #[tokio::test]
    async fn some_provider_resolves_to_the_configured_issuer() {
        let issuer = resolve_issuer_from_provider(Some(external_provider(
            "https://accounts.google.com",
        )))
        .await
        .unwrap();

        assert_eq!(issuer, "https://accounts.google.com");
    }
}
