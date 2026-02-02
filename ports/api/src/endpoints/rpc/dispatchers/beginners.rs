use super::super::{
    errors::{
        forbidden_owner_only, invalid_params, mapped_errors_to_jsonrpc_error,
        params_required,
    },
    params::{
        AcceptInvitationParams, CheckEmailPasswordValidityParams,
        CheckTokenAndActivateUserParams, CheckTokenAndResetPasswordParams,
        CreateAccountMetaParams, CreateConnectionStringParams,
        CreateDefaultAccountParams, CreateDefaultUserParams,
        DeleteAccountMetaParams, DeleteMyAccountParams, FetchMyProfileParams,
        FetchTenantPublicInfoParams, StartPasswordRedefinitionParams,
        TotpCheckTokenParams, TotpDisableParams, TotpFinishActivationParams,
        TotpStartActivationParams, UpdateAccountMetaParams,
        UpdateOwnAccountNameParams,
    },
    response_kind::{
        create_response_kind_to_result, delete_response_kind_to_result,
        fetch_many_response_kind_to_result, fetch_response_kind_to_result,
        updating_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::{
    dtos::MyceliumProfileData,
    middleware::{
        check_credentials_with_multi_identity_provider,
        parse_issuer_from_request,
    },
};

use actix_web::{web, HttpRequest};
use myc_core::{
    domain::dtos::{
        account::AccountMetaKey,
        email::Email,
        guest_role::Permission,
        profile::{LicensedResources, TenantsOwnership},
        security_group::PermissionedRole,
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::{
        account::{
            create_user_account, delete_my_account, get_my_account_details,
            update_own_account_name,
        },
        guest_user::accept_invitation,
        meta::{create_account_meta, delete_account_meta, update_account_meta},
        tenant::fetch_tenant_public_info,
        token::{create_connection_string, list_my_connection_strings},
        user::{
            check_email_password_validity, check_token_and_activate_user,
            check_token_and_reset_password, create_default_user,
            start_password_redefinition, totp_check_token, totp_disable,
            totp_finish_activation, totp_start_activation,
        },
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::responses::GatewayError;
use shaku::HasComponent;
use std::str::FromStr;
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
            let req =
                req.ok_or_else(|| invalid_params("Request context required"))?;
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
            fetch_response_kind_to_result(result)
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
            updating_response_kind_to_result(result)
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
            delete_response_kind_to_result(result)
        }
        "beginners.guests.acceptInvitation" => {
            let p: AcceptInvitationParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let permission = Permission::from_i32(p.permission);
            let result = accept_invitation(
                profile.to_profile(),
                p.account_id,
                p.guest_role_name,
                permission,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "beginners.meta.createAccountMeta" => {
            let p: CreateAccountMetaParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let key = AccountMetaKey::from_str(&p.key)
                .map_err(|_| invalid_params("The key is invalid"))?;
            let result = create_account_meta(
                profile.to_profile(),
                key,
                p.value,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            create_response_kind_to_result(result)
        }
        "beginners.meta.updateAccountMeta" => {
            let p: UpdateAccountMetaParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let key = AccountMetaKey::from_str(&p.key)
                .map_err(|_| invalid_params("The key is invalid"))?;
            let result = update_account_meta(
                profile.to_profile(),
                key,
                p.value,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        "beginners.meta.deleteAccountMeta" => {
            let p: DeleteAccountMetaParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let key = AccountMetaKey::from_str(&p.key)
                .map_err(|_| invalid_params("The key is invalid"))?;
            let result = delete_account_meta(
                profile.to_profile(),
                key,
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
        "beginners.profile.fetchMyProfile" => {
            let p: FetchMyProfileParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let with_url = p.with_url.unwrap_or(true);
            let mut profile_data = profile.clone();
            if with_url {
                if let Some(licensed_resources) =
                    profile_data.licensed_resources
                {
                    let resources = match licensed_resources {
                        LicensedResources::Urls(urls) => urls,
                        LicensedResources::Records(records) => records
                            .iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<String>>(),
                    };
                    profile_data.licensed_resources = if resources.is_empty() {
                        None
                    } else {
                        Some(LicensedResources::Urls(resources))
                    };
                }
                if let Some(tenants_ownership) = profile_data.tenants_ownership
                {
                    let ownerships = match tenants_ownership {
                        TenantsOwnership::Urls(urls) => urls,
                        TenantsOwnership::Records(records) => records
                            .iter()
                            .map(|r| r.to_string())
                            .collect::<Vec<String>>(),
                    };
                    profile_data.tenants_ownership = if ownerships.is_empty() {
                        None
                    } else {
                        Some(TenantsOwnership::Urls(ownerships))
                    };
                }
            }
            serde_json::to_value(profile_data.to_profile()).map_err(|e| {
                JsonRpcError {
                    code: types::codes::INTERNAL_ERROR,
                    message: e.to_string(),
                    data: None,
                }
            })
        }
        "beginners.tenants.fetchTenantPublicInfo" => {
            let p: FetchTenantPublicInfoParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = fetch_tenant_public_info(
                profile.to_profile(),
                p.tenant_id,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_response_kind_to_result(result)
        }
        "beginners.tokens.createConnectionString" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: CreateConnectionStringParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let roles: Option<Vec<PermissionedRole>> = p.roles.map(|v| {
                v.into_iter()
                    .map(|r| PermissionedRole {
                        name: r.name,
                        permission: r.permission.map(Permission::from_i32),
                    })
                    .collect()
            });
            let result = create_connection_string(
                profile.to_profile(),
                p.name,
                p.expiration,
                p.tenant_id,
                p.service_account_id,
                roles,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(
                serde_json::json!({ "connectionString": result }),
            )
            .map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.tokens.listMyConnectionStrings" => {
            let result = list_my_connection_strings(
                profile.to_profile(),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        "beginners.users.createDefaultUser" => {
            let req =
                req.ok_or_else(|| invalid_params("Request context required"))?;
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let provider = match parse_issuer_from_request(req.to_owned()).await
            {
                Err(GatewayError::Unauthorized(_)) => None,
                Err(err) => {
                    warn!("Invalid issuer: {:?}", err);
                    return Err(JsonRpcError {
                        code: types::codes::INVALID_REQUEST,
                        message: "Invalid issuer.".to_string(),
                        data: None,
                    });
                }
                Ok((issuer, _)) => Some(issuer),
            };
            let p: CreateDefaultUserParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let _ = create_default_user(
                p.email,
                p.first_name,
                p.last_name,
                p.password,
                provider,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(
                serde_json::json!({"message": "User created successfully"}),
            )
            .map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.users.checkTokenAndActivateUser" => {
            let p: CheckTokenAndActivateUserParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let result = check_token_and_activate_user(
                p.token,
                email,
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
        "beginners.users.startPasswordRedefinition" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: StartPasswordRedefinitionParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let _ = start_password_redefinition(
                email,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(true).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.users.checkTokenAndResetPassword" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: CheckTokenAndResetPasswordParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let _ = check_token_and_reset_password(
                p.token,
                email,
                p.new_password,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(true).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.users.checkEmailPasswordValidity" => {
            let p: CheckEmailPasswordValidityParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let (valid, user) = check_email_password_validity(
                email,
                p.password,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(serde_json::json!({
                "valid": valid,
                "user": user
            }))
            .map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.users.totpStartActivation" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: TotpStartActivationParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let (totp_url, totp_secret) = totp_start_activation(
                email,
                p.qr_code,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(serde_json::json!({
                "totpUrl": totp_url,
                "totpSecret": totp_secret
            }))
            .map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.users.totpFinishActivation" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: TotpFinishActivationParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let _ = totp_finish_activation(
                email,
                p.token,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(serde_json::json!({ "finished": true }))
                .map_err(|e| JsonRpcError {
                    code: types::codes::INTERNAL_ERROR,
                    message: e.to_string(),
                    data: None,
                })
        }
        "beginners.users.totpCheckToken" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: TotpCheckTokenParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let user = totp_check_token(
                email,
                p.token,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(user).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        "beginners.users.totpDisable" => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: TotpDisableParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let email = Email::from_string(p.email.clone())
                .map_err(|e| invalid_params(e.to_string()))?;
            let _ = totp_disable(
                email,
                p.token,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(serde_json::json!({})).map_err(|e| {
                JsonRpcError {
                    code: types::codes::INTERNAL_ERROR,
                    message: e.to_string(),
                    data: None,
                }
            })
        }
        _ => Err(JsonRpcError {
            code: types::codes::METHOD_NOT_FOUND,
            message: format!("Method not found: {}", method),
            data: None,
        }),
    }
}
