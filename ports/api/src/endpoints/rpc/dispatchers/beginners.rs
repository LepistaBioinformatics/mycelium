//! Dispatch of JSON-RPC methods for beginners scope (beginners.accounts.*).
//! Uses profile for get/update/delete; uses HttpRequest + credentials for createDefaultAccount.

use super::super::errors::{
    forbidden_owner_only, invalid_params, mapped_errors_to_jsonrpc_error,
    params_required,
};
use super::super::params::{
    CreateDefaultAccountParams, DeleteMyAccountParams, UpdateOwnAccountNameParams,
};
use super::super::types::{self, JsonRpcError};
use crate::dtos::MyceliumProfileData;
use crate::middleware::check_credentials_with_multi_identity_provider;

use actix_web::{web, HttpRequest};
use myc_core::{
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::account::{
        create_user_account, delete_my_account, get_my_account_details,
        update_own_account_name,
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;
use tracing::warn;

pub async fn dispatch_beginners(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    req: Option<&HttpRequest>,
    life_cycle_settings: Option<&web::Data<AccountLifeCycle>>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        "beginners.accounts.createDefaultAccount" => {
            let req = req.ok_or_else(|| invalid_params("Request context required"))?;
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let (email, external_provider) =
                check_credentials_with_multi_identity_provider(req.to_owned())
                    .await
                    .map_err(|err| {
                        warn!("check_credentials err: {:?}", err);
                        JsonRpcError {
                            code: types::codes::INVALID_REQUEST,
                            message: err.to_string(),
                            data: None,
                        }
                    })?;
            let issuer = if let Some(provider) = external_provider {
                match provider.issuer.async_get_or_error().await {
                    Ok(issuer) => issuer,
                    Err(err) => {
                        warn!("issuer err: {:?}", err);
                        return Err(JsonRpcError {
                            code: types::codes::INTERNAL_ERROR,
                            message: err.to_string(),
                            data: None,
                        });
                    }
                }
            } else {
                return Err(invalid_params("Invalid provider"));
            };
            let p: CreateDefaultAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = create_user_account(
                email,
                Some(issuer),
                p.name,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
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
        "beginners.accounts.getMyAccountDetails" => {
            let result = get_my_account_details(
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
        "beginners.accounts.updateOwnAccountName" => {
            let p: UpdateOwnAccountNameParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            if p.account_id != profile.acc_id {
                warn!(
                    "Account {} trying to update {}",
                    profile.acc_id, p.account_id
                );
                return Err(forbidden_owner_only());
            }
            let result = update_own_account_name(
                profile.to_profile(),
                p.name,
                Box::new(&*app_module.resolve_ref()),
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
        "beginners.accounts.deleteMyAccount" => {
            let p: DeleteMyAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            if p.account_id != profile.acc_id {
                warn!(
                    "Account {} trying to delete {}",
                    profile.acc_id, p.account_id
                );
                return Err(forbidden_owner_only());
            }
            let result = delete_my_account(
                profile.to_profile(),
                Box::new(&*app_module.resolve_ref()),
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
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
