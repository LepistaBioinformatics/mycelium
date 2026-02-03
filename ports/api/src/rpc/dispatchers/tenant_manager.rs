use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    params::{
        CreateSubscriptionManagerAccountParams,
        DeleteSubscriptionAccountParams, GetTenantDetailsParams,
        GuestUserToSubscriptionManagerAccountParams,
        RevokeUserGuestToSubscriptionManagerAccountParams,
        TenantManagerDeleteTagParams, TenantManagerRegisterTagParams,
        TenantManagerUpdateTagParams,
    },
    response_kind::{
        delete_response_kind_to_result, fetch_response_kind_to_result,
        get_or_create_response_kind_to_result,
        updating_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::dtos::{email::Email, guest_role::Permission, tag::Tag},
    models::AccountLifeCycle,
    use_cases::role_scoped::tenant_manager::{
        create_subscription_manager_account, delete_subscription_account,
        delete_tag, get_tenant_details,
        guest_user_to_subscription_manager_account, register_tag,
        revoke_user_guest_to_subscription_manager_account, update_tag,
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;

pub async fn dispatch_tenant_manager(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    life_cycle_settings: Option<&web::Data<AccountLifeCycle>>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        "tenantManager.accounts.createSubscriptionManagerAccount" => {
            let p: CreateSubscriptionManagerAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = create_subscription_manager_account(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        "tenantManager.accounts.deleteSubscriptionAccount" => {
            let p: DeleteSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_subscription_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "tenantManager.guests.guestUserToSubscriptionManagerAccount" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: GuestUserToSubscriptionManagerAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let permission = Permission::from_i32(p.permission);
            let result = guest_user_to_subscription_manager_account(
                profile.to_profile(),
                email,
                p.tenant_id,
                permission,
                p.account_id,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        "tenantManager.guests.revokeUserGuestToSubscriptionManagerAccount" => {
            let p: RevokeUserGuestToSubscriptionManagerAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let result = revoke_user_guest_to_subscription_manager_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                p.role_id,
                email,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "tenantManager.tags.registerTag" => {
            let p: TenantManagerRegisterTagParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let meta = p.meta.unwrap_or_default();
            let result = register_tag(
                profile.to_profile(),
                p.tenant_id,
                p.value,
                meta,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        "tenantManager.tags.updateTag" => {
            let p: TenantManagerUpdateTagParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let tag = Tag {
                id: p.tag_id,
                value: p.value,
                meta: p.meta,
            };
            let result = update_tag(
                profile.to_profile(),
                p.tenant_id,
                tag,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "tenantManager.tags.deleteTag" => {
            let p: TenantManagerDeleteTagParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_tag(
                profile.to_profile(),
                p.tenant_id,
                p.tag_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        "tenantManager.tenant.getTenantDetails" => {
            let p: GetTenantDetailsParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = get_tenant_details(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_response_kind_to_result(result)
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
