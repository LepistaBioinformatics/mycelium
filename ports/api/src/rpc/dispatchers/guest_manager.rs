use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::{
        CreateGuestRoleParams, DeleteGuestRoleParams,
        GuestManagerListGuestRolesParams, InsertRoleChildParams,
        RemoveRoleChildParams, UpdateGuestRoleNameAndDescriptionParams,
        UpdateGuestRolePermissionParams,
    },
    response_kind::{
        delete_response_kind_to_result, fetch_many_response_kind_to_result,
        get_or_create_response_kind_to_result,
        updating_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::dtos::guest_role::Permission,
    use_cases::role_scoped::guest_manager::guest_role::{
        create_guest_role, delete_guest_role, insert_role_child,
        list_guest_roles, remove_role_child,
        update_guest_role_name_and_description, update_guest_role_permission,
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;

pub async fn dispatch_guest_manager(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::GUEST_MANAGER_GUEST_ROLES_CREATE => {
            let p: CreateGuestRoleParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let permission = p.permission.map(Permission::from_i32);
            let result = create_guest_role(
                profile.to_profile(),
                p.name,
                p.description,
                permission,
                p.system,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        method_names::GUEST_MANAGER_GUEST_ROLES_LIST => {
            let p: GuestManagerListGuestRolesParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_guest_roles(
                profile.to_profile(),
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
        method_names::GUEST_MANAGER_GUEST_ROLES_DELETE => {
            let p: DeleteGuestRoleParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_guest_role(
                profile.to_profile(),
                p.guest_role_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        method_names::GUEST_MANAGER_GUEST_ROLES_UPDATE_NAME_AND_DESCRIPTION => {
            let p: UpdateGuestRoleNameAndDescriptionParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = update_guest_role_name_and_description(
                profile.to_profile(),
                p.name,
                p.description,
                p.guest_role_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::GUEST_MANAGER_GUEST_ROLES_UPDATE_PERMISSION => {
            let p: UpdateGuestRolePermissionParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let permission = Permission::from_i32(p.permission);
            let result = update_guest_role_permission(
                profile.to_profile(),
                p.guest_role_id,
                permission,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::GUEST_MANAGER_GUEST_ROLES_INSERT_ROLE_CHILD => {
            let p: InsertRoleChildParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = insert_role_child(
                profile.to_profile(),
                p.guest_role_id,
                p.child_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::GUEST_MANAGER_GUEST_ROLES_REMOVE_ROLE_CHILD => {
            let p: RemoveRoleChildParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = remove_role_child(
                profile.to_profile(),
                p.guest_role_id,
                p.child_id,
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
