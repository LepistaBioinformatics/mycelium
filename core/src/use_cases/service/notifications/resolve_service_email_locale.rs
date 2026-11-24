use crate::{
    domain::{dtos::tenant::TenantMetaKey, entities::TenantFetching},
    models::AccountLifeCycle,
    use_cases::support::resolve_notification_locale,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};
use uuid::Uuid;

/// Resolve the preferred locale of a service email
///
/// Request locale, then the locale of the string tenant, then the instance
/// locale, then `en-us`. The service templates fall back to their default
/// locale when the preferred one has no template.
#[tracing::instrument(name = "resolve_service_email_locale", skip_all)]
pub(crate) async fn resolve_service_email_locale(
    requested_locale: Option<String>,
    tenant_id: Option<Uuid>,
    life_cycle: &AccountLifeCycle,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<String, MappedErrors> {
    let tenant_locale = match (&requested_locale, tenant_id) {
        (None, Some(tenant_id)) => {
            fetch_tenant_locale(tenant_id, *tenant_fetching_repo).await?
        }
        _ => None,
    };

    resolve_notification_locale(requested_locale, tenant_locale, life_cycle)
        .await
}

async fn fetch_tenant_locale(
    tenant_id: Uuid,
    tenant_fetching_repo: &dyn TenantFetching,
) -> Result<Option<String>, MappedErrors> {
    let FetchResponseKind::Found(tenant) = tenant_fetching_repo
        .get_tenant_public_by_id(tenant_id)
        .await?
    else {
        return Ok(None);
    };

    Ok(tenant
        .meta
        .and_then(|meta| meta.get(&TenantMetaKey::Locale).cloned()))
}
