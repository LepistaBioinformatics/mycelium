use async_trait::async_trait;
use myc_core::domain::entities::{
    LinkConsentRegistration, LinkRequestRegistration, RequestNonceRegistration,
};
use mycelium_base::utils::errors::MappedErrors;
use std::sync::Arc;

/// Rows a federation round deletes once they expire
#[async_trait]
pub(crate) trait ExpiredRowsPurge: Send + Sync {
    /// What is purged, for the logs
    fn rows(&self) -> &'static str;

    async fn purge_expired(&self) -> Result<u64, MappedErrors>;
}

/// The single-use `jti`s of signed peer requests (D-9)
pub(crate) struct RequestNoncePurge(
    pub(crate) Arc<dyn RequestNonceRegistration>,
);

/// The consumer's link initiations (FED-42)
pub(crate) struct LinkRequestPurge(pub(crate) Arc<dyn LinkRequestRegistration>);

/// The issuer's pending link consents (FED-46)
pub(crate) struct LinkConsentPurge(pub(crate) Arc<dyn LinkConsentRegistration>);

#[async_trait]
impl ExpiredRowsPurge for RequestNoncePurge {
    fn rows(&self) -> &'static str {
        "request nonces"
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        self.0.purge_expired().await
    }
}

#[async_trait]
impl ExpiredRowsPurge for LinkRequestPurge {
    fn rows(&self) -> &'static str {
        "link requests"
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        self.0.purge_expired().await
    }
}

#[async_trait]
impl ExpiredRowsPurge for LinkConsentPurge {
    fn rows(&self) -> &'static str {
        "link consents"
    }

    async fn purge_expired(&self) -> Result<u64, MappedErrors> {
        self.0.purge_expired().await
    }
}
