use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::FetchResourceAuditTrailParams,
    response_kind::fetch_many_response_kind_to_result,
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use crate::models::active_backend_modules::SqlAppModule;
use actix_web::web;
use myc_core::{
    domain::{
        dtos::{
            account_type::AccountType, related_accounts::RelatedAccounts,
            resource_audit_log::ResourceAuditResourceType,
        },
        entities::AccountFetching,
    },
    use_cases::shared::audit::fetch_resource_audit_trail,
};
use mycelium_base::entities::FetchResponseKind;
use shaku::HasComponent;
use uuid::Uuid;

/// Mirrors `ResourceAuditResourceType`'s camelCase serde representation --
/// REST accepts the same strings via `web::Query`'s enum deserialization,
/// RPC parses them manually here for the same reason `managers.rs` parses
/// `actor` manually (`schemars::JsonSchema` isn't derived on the domain
/// enum, so the RPC param type stays a plain `String`).
fn parse_resource_type(s: &str) -> Option<ResourceAuditResourceType> {
    match s {
        "account" => Some(ResourceAuditResourceType::Account),
        "accountMeta" => Some(ResourceAuditResourceType::AccountMeta),
        "user" => Some(ResourceAuditResourceType::User),
        "tenant" => Some(ResourceAuditResourceType::Tenant),
        "tenantMeta" => Some(ResourceAuditResourceType::TenantMeta),
        "guestRole" => Some(ResourceAuditResourceType::GuestRole),
        "webhook" => Some(ResourceAuditResourceType::Webhook),
        _ => None,
    }
}

pub async fn dispatch_audit(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::AUDIT_RESOURCE_TRAIL_FETCH => {
            let p: FetchResourceAuditTrailParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;

            let resource_type = parse_resource_type(&p.resource_type)
                .ok_or_else(|| {
                    invalid_params(
                        "resourceType must be one of: account, accountMeta, \
                         user, tenant, tenantMeta, guestRole, webhook",
                    )
                })?;

            let (tenant_id, resource_owner_account_id) = resolve_audit_context(
                &resource_type,
                p.resource_id,
                app_module,
            )
            .await?;

            let result = fetch_resource_audit_trail(
                profile.to_profile(),
                resource_type,
                p.resource_id,
                tenant_id,
                resource_owner_account_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;

            fetch_many_response_kind_to_result(result)
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}

// ? ---------------------------------------------------------------------------
// ? Private helpers
//
// Mirrors the REST handler's `resolve_audit_context`/`account_tenant_id`
// (`rest/audit/resource_audit_trail_endpoints.rs`) -- same lookup, same
// reasoning, kept independent per this codebase's REST/RPC parity
// convention (each side owns its own request-shape-to-use-case-input
// mapping; see `STATE.md`'s "RPC ↔ REST Audit" section).
// ? ---------------------------------------------------------------------------

async fn resolve_audit_context(
    resource_type: &ResourceAuditResourceType,
    resource_id: Uuid,
    app_module: &web::Data<SqlAppModule>,
) -> Result<(Option<Uuid>, Option<Uuid>), JsonRpcError> {
    match resource_type {
        ResourceAuditResourceType::Tenant => Ok((Some(resource_id), None)),
        ResourceAuditResourceType::Account => {
            resolve_account_audit_context(resource_id, app_module).await
        }
        _ => Ok((None, None)),
    }
}

async fn resolve_account_audit_context(
    account_id: Uuid,
    app_module: &web::Data<SqlAppModule>,
) -> Result<(Option<Uuid>, Option<Uuid>), JsonRpcError> {
    let account_fetching_repo: &dyn AccountFetching = app_module.resolve_ref();

    let account = match account_fetching_repo
        .get(account_id, RelatedAccounts::HasStaffPrivileges)
        .await
    {
        Ok(FetchResponseKind::Found(account)) => account,
        Ok(FetchResponseKind::NotFound(_)) => {
            return Err(invalid_params("Invalid account ID"))
        }
        Err(err) => return Err(mapped_errors_to_jsonrpc_error(err)),
    };

    Ok((account_tenant_id(&account.account_type), Some(account_id)))
}

fn account_tenant_id(account_type: &AccountType) -> Option<Uuid> {
    match account_type {
        AccountType::Subscription { tenant_id } => Some(*tenant_id),
        AccountType::RoleAssociated { tenant_id, .. } => Some(*tenant_id),
        AccountType::TenantManager { tenant_id } => Some(*tenant_id),
        _ => None,
    }
}
