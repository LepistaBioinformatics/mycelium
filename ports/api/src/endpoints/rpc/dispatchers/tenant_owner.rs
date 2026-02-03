use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    params::{
        CreateManagementAccountParams, CreateTenantMetaParams,
        DeleteTenantManagerAccountParams, DeleteTenantMetaParams,
        GuestTenantOwnerParams, RevokeTenantOwnerParams,
        UpdateTenantArchivingStatusParams,
        UpdateTenantNameAndDescriptionParams, UpdateTenantTrashingStatusParams,
        UpdateTenantVerifyingStatusParams,
    },
    response_kind::{
        create_response_kind_to_result, delete_response_kind_to_result,
        get_or_create_response_kind_to_result,
        updating_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::dtos::{email::Email, tenant::TenantMetaKey},
    use_cases::role_scoped::tenant_owner::{
        create_management_account, create_tenant_meta,
        delete_tenant_manager_account, delete_tenant_meta, guest_tenant_owner,
        revoke_tenant_owner, update_tenant_archiving_status,
        update_tenant_name_and_description, update_tenant_trashing_status,
        update_tenant_verifying_status,
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;
use std::str::FromStr;

pub async fn dispatch_tenant_owner(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        "tenantOwner.accounts.createManagementAccount" => {
            let p: CreateManagementAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = create_management_account(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        "tenantOwner.accounts.deleteTenantManagerAccount" => {
            let p: DeleteTenantManagerAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_tenant_manager_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "tenantOwner.meta.createTenantMeta" => {
            let p: CreateTenantMetaParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let key = TenantMetaKey::from_str(&p.key)
                .map_err(|e| invalid_params(e))?;
            let result = create_tenant_meta(
                profile.to_profile(),
                p.tenant_id,
                key,
                p.value,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            create_response_kind_to_result(result)
        }
        "tenantOwner.meta.deleteTenantMeta" => {
            let p: DeleteTenantMetaParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let key = TenantMetaKey::from_str(&p.key)
                .map_err(|e| invalid_params(e))?;
            let result = delete_tenant_meta(
                profile.to_profile(),
                p.tenant_id,
                key,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "tenantOwner.owner.guestTenantOwner" => {
            let p: GuestTenantOwnerParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let result = guest_tenant_owner(
                profile.to_profile(),
                email,
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            create_response_kind_to_result(result)
        }
        "tenantOwner.owner.revokeTenantOwner" => {
            let p: RevokeTenantOwnerParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let result = revoke_tenant_owner(
                profile.to_profile(),
                email,
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "tenantOwner.tenant.updateTenantNameAndDescription" => {
            let p: UpdateTenantNameAndDescriptionParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = update_tenant_name_and_description(
                profile.to_profile(),
                p.tenant_id,
                p.name,
                p.description,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "tenantOwner.tenant.updateTenantArchivingStatus" => {
            let p: UpdateTenantArchivingStatusParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = update_tenant_archiving_status(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "tenantOwner.tenant.updateTenantTrashingStatus" => {
            let p: UpdateTenantTrashingStatusParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = update_tenant_trashing_status(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "tenantOwner.tenant.updateTenantVerifyingStatus" => {
            let p: UpdateTenantVerifyingStatusParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = update_tenant_verifying_status(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
