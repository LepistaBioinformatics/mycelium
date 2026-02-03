//! Dispatcher JSON-RPC para o escopo subscriptionsManager (accounts, guests, guestRoles, tags).

use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::{
        CreateRoleAssociatedAccountParams, CreateSubscriptionAccountParams,
        DeleteTagParams, GetAccountDetailsParams,
        GuestUserToSubscriptionAccountParams, ListAccountsByTypeParams,
        ListGuestOnSubscriptionAccountParams,
        ListLicensedAccountsOfEmailParams, PropagateSubscriptionAccountParams,
        RegisterTagParams, RevokeUserGuestToSubscriptionAccountParams,
        SubscriptionsManagerFetchGuestRoleDetailsParams,
        SubscriptionsManagerListGuestRolesParams,
        UpdateAccountNameAndFlagsParams,
        UpdateFlagsFromSubscriptionAccountParams, UpdateTagParams,
    },
    response_kind::{
        delete_response_kind_to_result, fetch_many_response_kind_to_result,
        fetch_response_kind_to_result, get_or_create_response_kind_to_result,
        updating_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::{
        actors::SystemActor,
        dtos::{
            account::VerboseStatus, account_type::AccountType, email::Email,
            guest_role::Permission, security_group::PermissionedRole, tag::Tag,
        },
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::subscriptions_manager::{
        account::{
            create_role_associated_account, create_subscription_account,
            get_account_details, list_accounts_by_type,
            propagate_existing_subscription_account,
            update_account_name_and_flags,
        },
        guest::{
            guest_user_to_subscription_account,
            list_guest_on_subscription_account,
            list_licensed_accounts_of_email,
            revoke_user_guest_to_subscription_account,
            update_flags_from_subscription_account,
        },
        guest_role::{fetch_guest_role_details, list_guest_roles},
        tag::{delete_tag, register_tag, update_tag},
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;
use std::str::FromStr;
use uuid::Uuid;

fn parse_actor(s: &str) -> Result<SystemActor, JsonRpcError> {
    let kebab = match s {
        "gatewayManager" => "gateway-manager",
        "guestsManager" => "guests-manager",
        "systemManager" => "system-manager",
        other => other,
    };
    SystemActor::from_str(kebab)
        .map_err(|_| invalid_params(format!("Unknown actor: {}", s)))
}

fn parse_account_type_from_params(
    p: &ListAccountsByTypeParams,
) -> Result<Option<AccountType>, JsonRpcError> {
    let s = match &p.account_type {
        None => return Ok(None),
        Some(x) => x.as_str(),
    };
    let tenant_id = p.tenant_id;
    let actor_str = p.actor.as_deref();
    let role_name = p.role_name.as_deref().unwrap_or("").to_string();
    let read_role_id = p.read_role_id.unwrap_or(Uuid::nil());
    let write_role_id = p.write_role_id.unwrap_or(Uuid::nil());

    let account_type = match s {
        "Staff" | "staff" => AccountType::Staff,
        "Manager" | "manager" => AccountType::Manager,
        "User" | "user" => AccountType::User,
        "Subscription" | "subscription" => {
            let tid = tenant_id.ok_or_else(|| {
                invalid_params("tenant_id required for Subscription")
            })?;
            AccountType::Subscription { tenant_id: tid }
        }
        "TenantManager" | "tenantManager" | "tenant_manager" => {
            let tid = tenant_id.ok_or_else(|| {
                invalid_params("tenant_id required for TenantManager")
            })?;
            AccountType::TenantManager { tenant_id: tid }
        }
        "ActorAssociated" | "actorAssociated" | "actor_associated" => {
            let actor = actor_str.ok_or_else(|| {
                invalid_params("actor required for ActorAssociated")
            })?;
            AccountType::ActorAssociated {
                actor: parse_actor(actor)?,
            }
        }
        "RoleAssociated" | "roleAssociated" | "role_associated" => {
            let tid = tenant_id.ok_or_else(|| {
                invalid_params("tenant_id required for RoleAssociated")
            })?;
            AccountType::RoleAssociated {
                tenant_id: tid,
                role_name,
                read_role_id,
                write_role_id,
            }
        }
        _ => {
            return Err(invalid_params(format!("Unknown account_type: {}", s)))
        }
    };
    Ok(Some(account_type))
}

fn status_to_flags(
    status: Option<&String>,
) -> Result<
    (Option<bool>, Option<bool>, Option<bool>, Option<bool>),
    JsonRpcError,
> {
    let s = match status {
        None => return Ok((None, None, None, None)),
        Some(x) => x.as_str(),
    };
    let v = VerboseStatus::from_str(s).map_err(|_| {
        invalid_params(format!(
            "status must be one of: unverified, verified, inactive, archived, deleted"
        ))
    })?;
    let f = v.to_flags().map_err(mapped_errors_to_jsonrpc_error)?;
    Ok((f.is_active, f.is_checked, f.is_archived, f.is_deleted))
}

fn role_params_to_permissioned_roles(
    roles: Option<Vec<super::super::params::subscriptions_manager::RoleParam>>,
) -> Option<Vec<PermissionedRole>> {
    roles.map(|v| {
        v.into_iter()
            .map(|r| PermissionedRole {
                name: r.name,
                permission: r.permission.map(Permission::from_i32),
            })
            .collect()
    })
}

pub async fn dispatch_subscriptions_manager(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    life_cycle_settings: Option<&web::Data<AccountLifeCycle>>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_CREATE_SUBSCRIPTION_ACCOUNT => {
            let p: CreateSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let account = create_subscription_account(
                profile.to_profile(),
                p.tenant_id,
                p.name,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(account).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_CREATE_ROLE_ASSOCIATED_ACCOUNT => {
            let p: CreateRoleAssociatedAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = create_role_associated_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_name,
                p.role_name,
                p.role_description,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_LIST => {
            let p: ListAccountsByTypeParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let account_type = parse_account_type_from_params(&p)?;
            let (is_active, is_checked, is_archived, is_deleted) =
                status_to_flags(p.status.as_ref())?;
            let result = list_accounts_by_type(
                profile.to_profile(),
                p.tenant_id,
                p.term,
                p.is_owner_active,
                is_active,
                is_checked,
                is_archived,
                is_deleted,
                account_type,
                p.tag_value,
                p.page_size,
                p.skip,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_GET => {
            let p: GetAccountDetailsParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = get_account_details(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_UPDATE_NAME_AND_FLAGS => {
            let p: UpdateAccountNameAndFlagsParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = update_account_name_and_flags(
                profile.to_profile(),
                p.account_id,
                p.tenant_id,
                p.name,
                p.is_active,
                p.is_checked,
                p.is_archived,
                p.is_system_account,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_ACCOUNTS_PROPAGATE_SUBSCRIPTION_ACCOUNT => {
            let p: PropagateSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let account = propagate_existing_subscription_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(account).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        method_names::SUBSCRIPTIONS_MANAGER_GUESTS_LIST_LICENSED_ACCOUNTS_OF_EMAIL => {
            let p: ListLicensedAccountsOfEmailParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email =
                Email::from_string(p.email.clone()).map_err(|e| invalid_params(e.to_string()))?;
            let roles = role_params_to_permissioned_roles(p.roles);
            let result = list_licensed_accounts_of_email(
                profile.to_profile(),
                p.tenant_id,
                email,
                roles,
                p.was_verified,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_GUESTS_GUEST_USER_TO_SUBSCRIPTION_ACCOUNT => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: GuestUserToSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email =
                Email::from_string(p.email.clone()).map_err(|e| invalid_params(e.to_string()))?;
            let result = guest_user_to_subscription_account(
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
        method_names::SUBSCRIPTIONS_MANAGER_GUESTS_UPDATE_FLAGS_FROM_SUBSCRIPTION_ACCOUNT => {
            let p: UpdateFlagsFromSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let permit_flags = p.permit_flags.unwrap_or_default();
            let deny_flags = p.deny_flags.unwrap_or_default();
            let result = update_flags_from_subscription_account(
                profile.to_profile(),
                p.tenant_id,
                p.role_id,
                p.account_id,
                permit_flags,
                deny_flags,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_GUESTS_REVOKE_USER_GUEST_TO_SUBSCRIPTION_ACCOUNT => {
            let p: RevokeUserGuestToSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = revoke_user_guest_to_subscription_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                p.role_id,
                p.email,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            delete_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_GUESTS_LIST_GUEST_ON_SUBSCRIPTION_ACCOUNT => {
            let p: ListGuestOnSubscriptionAccountParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = list_guest_on_subscription_account(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                p.page_size,
                p.skip,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_GUEST_ROLES_LIST => {
            let p: SubscriptionsManagerListGuestRolesParams = params
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
        method_names::SUBSCRIPTIONS_MANAGER_GUEST_ROLES_GET => {
            let p: SubscriptionsManagerFetchGuestRoleDetailsParams =
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
        method_names::SUBSCRIPTIONS_MANAGER_TAGS_CREATE => {
            let p: RegisterTagParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let meta = p.meta.unwrap_or_default();
            let result = register_tag(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                p.value,
                meta,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            get_or_create_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_TAGS_UPDATE => {
            let p: UpdateTagParams =
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
                p.account_id,
                tag,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::SUBSCRIPTIONS_MANAGER_TAGS_DELETE => {
            let p: DeleteTagParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_tag(
                profile.to_profile(),
                p.tenant_id,
                p.account_id,
                p.tag_id,
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
