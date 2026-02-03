use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    params::{
        CreateSystemAccountParams, CreateTenantParams, DeleteTenantParams,
        ExcludeTenantOwnerParams, IncludeTenantOwnerParams, ListTenantParams,
    },
    response_kind::{
        create_response_kind_to_result, delete_response_kind_to_result,
        fetch_many_response_kind_to_result,
        get_or_create_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::domain::dtos::tenant::TenantMetaKey;
use myc_core::use_cases::super_users::managers::{
    create_system_account, create_system_roles, create_tenant, delete_tenant,
    exclude_tenant_owner, include_tenant_owner, list_tenant,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::SystemActor;
use shaku::HasComponent;
use std::str::FromStr;

fn parse_system_actor(s: &str) -> Option<SystemActor> {
    match s {
        "gatewayManager" => Some(SystemActor::GatewayManager),
        "guestsManager" => Some(SystemActor::GuestsManager),
        "systemManager" => Some(SystemActor::SystemManager),
        _ => None,
    }
}

pub async fn dispatch_managers(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        "managers.accounts.createSystemAccount" => {
            let p: CreateSystemAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let actor = parse_system_actor(&p.actor).ok_or_else(|| {
                invalid_params(
                    "actor must be gatewayManager, guestsManager, or systemManager",
                )
            })?;
            let result = create_system_account(
                profile.to_profile(),
                p.name,
                actor,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        "managers.guestRoles.createSystemRoles" => {
            let result = create_system_roles(
                profile.to_profile(),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(result).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "managers.tenants.createTenant" => {
            let p: CreateTenantParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = create_tenant(
                profile.to_profile(),
                p.name,
                p.description,
                p.owner_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            create_response_kind_to_result(result)
        }
        "managers.tenants.listTenant" => {
            let p: ListTenantParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let tag = match p.tag.as_ref() {
                Some(t) => match t.split_once('=') {
                    Some((k, v)) => Some((k.to_string(), v.to_string())),
                    None => return Err(invalid_params("Invalid tag format")),
                },
                None => None,
            };
            let metadata = match p.metadata.as_ref() {
                Some(m) => match m.split_once('=') {
                    Some((k, v)) => {
                        let key = TenantMetaKey::from_str(k).map_err(|_| {
                            invalid_params("Invalid metadata key")
                        })?;
                        Some((key, v.to_string()))
                    }
                    None => {
                        return Err(invalid_params("Invalid metadata format"))
                    }
                },
                None => None,
            };
            let result = list_tenant(
                profile.to_profile(),
                p.name,
                p.owner,
                metadata,
                tag,
                p.page_size,
                p.skip,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        "managers.tenants.deleteTenant" => {
            let p: DeleteTenantParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_tenant(
                profile.to_profile(),
                p.id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "managers.tenants.includeTenantOwner" => {
            let p: IncludeTenantOwnerParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = include_tenant_owner(
                profile.to_profile(),
                p.id,
                p.owner_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            create_response_kind_to_result(result)
        }
        "managers.tenants.excludeTenantOwner" => {
            let p: ExcludeTenantOwnerParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = exclude_tenant_owner(
                profile.to_profile(),
                p.id,
                p.owner_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
