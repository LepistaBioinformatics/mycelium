use super::{
    super::{
        errors::{invalid_params, mapped_errors_to_jsonrpc_error},
        method_names,
        params::{
            CreateTranslationRuleParams, ListTranslationRulesParams,
            SetTranslationRuleEnabledParams, TranslationRuleParams,
        },
        types::JsonRpcError,
    },
    federation::{method_not_found, parse, to_result},
};
use crate::models::active_backend_modules::SqlAppModule;

use myc_core::{
    domain::{
        dtos::{
            federation::{
                IssuerGrantMatch, LocalGrant, NewGrantTranslationRule,
            },
            guest_role::Permission,
            profile::Profile,
        },
        entities::FederationPeerRead,
    },
    use_cases::federation::{
        create_translation_rule, delete_translation_rule,
        list_translation_rules, set_translation_rule_enabled, TranslationPorts,
    },
};
use mycelium_base::entities::FetchResponseKind;
use shaku::HasComponent;
use std::str::FromStr;
use uuid::Uuid;

/// `federation.translations.*`: staff manage grant translation rules (FED-48)
pub(super) async fn dispatch_federation_translations(
    profile: &Profile,
    app_module: &SqlAppModule,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    let ports = ports(app_module);

    match method {
        method_names::FEDERATION_TRANSLATIONS_LIST => {
            let peer = parse::<ListTranslationRulesParams>(params)?;
            let peer_id = match peer.peer_instance_id {
                Some(instance_id) => {
                    Some(peer_id(app_module, instance_id).await?)
                }
                None => None,
            };
            to_result(list_translation_rules(profile, peer_id, ports).await)
        }
        method_names::FEDERATION_TRANSLATIONS_CREATE => {
            let rule = new_rule(app_module, parse(params)?).await?;
            to_result(create_translation_rule(profile, rule, ports).await)
        }
        method_names::FEDERATION_TRANSLATIONS_SET_ENABLED => {
            let p = parse::<SetTranslationRuleEnabledParams>(params)?;
            to_result(
                set_translation_rule_enabled(
                    profile, p.rule_id, p.enabled, ports,
                )
                .await,
            )
        }
        method_names::FEDERATION_TRANSLATIONS_DELETE => {
            let rule_id = parse::<TranslationRuleParams>(params)?.rule_id;
            to_result(delete_translation_rule(profile, rule_id, ports).await)
        }
        _ => Err(method_not_found(method)),
    }
}

fn ports(app_module: &SqlAppModule) -> TranslationPorts<'_> {
    TranslationPorts {
        read: app_module.resolve_ref(),
        write: app_module.resolve_ref(),
        peers: app_module.resolve_ref(),
        audit: app_module.resolve_ref(),
    }
}

async fn new_rule(
    app_module: &SqlAppModule,
    p: CreateTranslationRuleParams,
) -> Result<NewGrantTranslationRule, JsonRpcError> {
    Ok(NewGrantTranslationRule {
        peer_id: peer_id(app_module, p.peer_instance_id).await?,
        issuer: IssuerGrantMatch {
            tenant_id: p.issuer_tenant_id,
            role: p.issuer_role,
            min_perm: permission(&p.issuer_min_perm)?,
        },
        local: LocalGrant {
            account_id: p.local_account_id,
            guest_role_id: p.local_guest_role_id,
            perm: permission(&p.local_perm)?,
        },
    })
}

async fn peer_id(
    app_module: &SqlAppModule,
    instance_id: Uuid,
) -> Result<Uuid, JsonRpcError> {
    let peers: &dyn FederationPeerRead = app_module.resolve_ref();

    match peers
        .get_by_instance_id(instance_id)
        .await
        .map_err(mapped_errors_to_jsonrpc_error)?
    {
        FetchResponseKind::Found(peer) => Ok(peer.id),
        FetchResponseKind::NotFound(_) => Err(invalid_params(format!(
            "no peer with instance id {instance_id}"
        ))),
    }
}

fn permission(value: &str) -> Result<Permission, JsonRpcError> {
    Permission::from_str(value)
        .map_err(|_| invalid_params("a permission is read or write"))
}
