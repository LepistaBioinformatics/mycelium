use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::{
        DowngradeAccountPrivilegesParams, UpgradeAccountPrivilegesParams,
    },
    response_kind::updating_response_kind_to_result,
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::dtos::account_type::AccountType,
    use_cases::super_users::staff::account::{
        downgrade_account_privileges, upgrade_account_privileges,
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;

fn parse_upgrade_target(s: &str) -> Result<AccountType, JsonRpcError> {
    match s {
        "Staff" | "staff" => Ok(AccountType::Staff),
        "Manager" | "manager" => Ok(AccountType::Manager),
        _ => Err(invalid_params(
            "to must be Staff or Manager for upgrade".to_string(),
        )),
    }
}

fn parse_downgrade_target(s: &str) -> Result<AccountType, JsonRpcError> {
    match s {
        "Manager" | "manager" => Ok(AccountType::Manager),
        "User" | "user" => Ok(AccountType::User),
        _ => Err(invalid_params(
            "to must be Manager or User for downgrade".to_string(),
        )),
    }
}

pub async fn dispatch_staff(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::STAFF_ACCOUNTS_UPGRADE_PRIVILEGES => {
            let p: UpgradeAccountPrivilegesParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let target = parse_upgrade_target(&p.to)?;
            let result = upgrade_account_privileges(
                profile.to_profile(),
                p.account_id,
                target,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::STAFF_ACCOUNTS_DOWNGRADE_PRIVILEGES => {
            let p: DowngradeAccountPrivilegesParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let target = parse_downgrade_target(&p.to)?;
            let result = downgrade_account_privileges(
                profile.to_profile(),
                p.account_id,
                target,
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
