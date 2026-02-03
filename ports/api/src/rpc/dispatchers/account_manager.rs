use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::{
        FetchGuestRoleDetailsParams, GuestToChildrenAccountParams,
        ListGuestRolesParams,
    },
    response_kind::{
        fetch_many_response_kind_to_result, fetch_response_kind_to_result,
        get_or_create_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::dtos::email::Email,
    models::AccountLifeCycle,
    use_cases::role_scoped::account_manager::{
        guest::guest_to_children_account,
        guest_role::{fetch_guest_role_details, list_guest_roles},
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;

pub async fn dispatch_account_manager(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    life_cycle_settings: Option<&web::Data<AccountLifeCycle>>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::ACCOUNT_MANAGER_GUESTS_GUEST_TO_CHILDREN_ACCOUNT => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: GuestToChildrenAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let result = guest_to_children_account(
                profile.to_profile(),
                p.tenant_id,
                email,
                p.role_id,
                p.account_id,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        method_names::ACCOUNT_MANAGER_GUEST_ROLES_LIST_GUEST_ROLES => {
            let p: ListGuestRolesParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_guest_roles(
                profile.to_profile(),
                p.tenant_id,
                p.name,
                p.slug,
                p.system,
                p.page_size,
                p.skip,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        method_names::ACCOUNT_MANAGER_GUEST_ROLES_FETCH_GUEST_ROLE_DETAILS => {
            let p: FetchGuestRoleDetailsParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = fetch_guest_role_details(
                profile.to_profile(),
                p.tenant_id,
                p.id,
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
