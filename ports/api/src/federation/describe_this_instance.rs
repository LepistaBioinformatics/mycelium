use super::PreparedFederation;

use myc_core::{
    domain::dtos::federation::{FederationPublicJwk, InstanceSelfDescription},
    use_cases::federation::describe_instance,
};

/// The self-description this instance publishes, from what boot prepared
pub(crate) fn describe_this_instance(
    prepared: &PreparedFederation,
) -> InstanceSelfDescription {
    let keys = prepared
        .signing
        .public_keys
        .iter()
        .filter_map(|key| {
            serde_json::from_value::<FederationPublicJwk>(key.to_json()).ok()
        })
        .collect();

    describe_instance(
        prepared.instance_id,
        &prepared.config,
        env!("CARGO_PKG_VERSION"),
        keys,
    )
}
