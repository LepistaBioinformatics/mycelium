use schemars::JsonSchema;
use serde::Deserialize;
use uuid::Uuid;

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ListTranslationRulesParams {
    #[schemars(
        description = "Only the rules of the peer with this instanceId"
    )]
    pub peer_instance_id: Option<Uuid>,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTranslationRuleParams {
    #[schemars(
        description = "The instanceId of the peer whose grants it translates"
    )]
    pub peer_instance_id: Uuid,
    #[schemars(description = "The issuer tenant the grant must be on")]
    pub issuer_tenant_id: Uuid,
    #[schemars(description = "The issuer role the grant must carry")]
    pub issuer_role: String,
    #[schemars(
        description = "The least issuer permission the grant must have: read or write"
    )]
    pub issuer_min_perm: String,
    #[schemars(description = "A local tenant account, not a system account")]
    pub local_account_id: Uuid,
    #[schemars(description = "A local guest role")]
    pub local_guest_role_id: Uuid,
    #[schemars(
        description = "The local permission: read or write; never above the issuer's"
    )]
    pub local_perm: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TranslationRuleParams {
    #[schemars(description = "The id of the translation rule")]
    pub rule_id: Uuid,
}

#[derive(Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SetTranslationRuleEnabledParams {
    #[schemars(description = "The id of the translation rule")]
    pub rule_id: Uuid,
    pub enabled: bool,
}
