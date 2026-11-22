use super::PreparedFederation;

use myc_core::{
    domain::dtos::federation::{FederationPublicJwk, InstanceSelfDescription},
    use_cases::federation::describe_instance,
};
use myc_http_tools::settings::ROLE_CATALOG_PATH;

/// The self-description this instance publishes, from what boot prepared
///
/// `role_catalog_published` adds the catalog URL (D-10); the caller reads
/// the catalog mode, since it lives in the database.
///
pub(crate) fn describe_this_instance(
    prepared: &PreparedFederation,
    role_catalog_published: bool,
) -> InstanceSelfDescription {
    let keys = prepared
        .signing
        .public_keys
        .iter()
        .filter_map(|key| {
            serde_json::from_value::<FederationPublicJwk>(key.to_json()).ok()
        })
        .collect();
    let base_url = prepared.config.base_url.trim_end_matches('/');
    let role_catalog_url = role_catalog_published
        .then(|| format!("{base_url}{ROLE_CATALOG_PATH}"));

    describe_instance(
        prepared.instance_id,
        &prepared.config,
        env!("CARGO_PKG_VERSION"),
        keys,
        role_catalog_url,
    )
}
