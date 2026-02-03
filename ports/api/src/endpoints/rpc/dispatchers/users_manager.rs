use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    params::UserManagerAccountIdParams,
    response_kind::updating_response_kind_to_result,
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::use_cases::role_scoped::users_manager::account::{
    change_account_activation_status, change_account_approval_status,
    change_account_archival_status,
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;

pub async fn dispatch_users_manager(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        "userManager.account.approveAccount" => {
            let p: UserManagerAccountIdParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = change_account_approval_status(
                profile.to_profile(),
                p.account_id,
                true,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "userManager.account.disapproveAccount" => {
            let p: UserManagerAccountIdParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = change_account_approval_status(
                profile.to_profile(),
                p.account_id,
                false,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "userManager.account.activateAccount" => {
            let p: UserManagerAccountIdParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = change_account_activation_status(
                profile.to_profile(),
                p.account_id,
                true,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "userManager.account.deactivateAccount" => {
            let p: UserManagerAccountIdParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = change_account_activation_status(
                profile.to_profile(),
                p.account_id,
                false,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "userManager.account.archiveAccount" => {
            let p: UserManagerAccountIdParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = change_account_archival_status(
                profile.to_profile(),
                p.account_id,
                true,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "userManager.account.unarchiveAccount" => {
            let p: UserManagerAccountIdParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = change_account_archival_status(
                profile.to_profile(),
                p.account_id,
                false,
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
