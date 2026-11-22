use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct FederationPeerParams {
    #[schemars(description = "The instanceId the peer was declared with")]
    pub instance_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetFederationEnabledParams {
    #[schemars(
        description = "true turns federation on (refused without [core.federation]); false is the kill-switch"
    )]
    pub enabled: bool,
}
