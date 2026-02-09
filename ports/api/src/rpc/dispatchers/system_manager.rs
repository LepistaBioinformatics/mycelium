use super::super::{
    errors::{invalid_params, mapped_errors_to_jsonrpc_error, params_required},
    method_names,
    params::{
        DeleteErrorCodeParams, DeleteWebhookParams, GetErrorCodeParams,
        ListErrorCodesParams, ListWebhooksParams, RegisterErrorCodeParams,
        RegisterWebhookParams, UpdateErrorCodeMessageAndDetailsParams,
        UpdateWebhookParams,
    },
    response_kind::{
        create_response_kind_to_result, delete_response_kind_to_result,
        fetch_many_response_kind_to_result, fetch_response_kind_to_result,
        updating_response_kind_to_result,
    },
    types::{self, JsonRpcError},
};
use crate::dtos::MyceliumProfileData;

use actix_web::web;
use myc_core::{
    domain::dtos::{
        http::HttpMethod,
        http_secret::HttpSecret,
        webhook::{WebHook, WebHookTrigger},
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::system_manager::{
        error_codes::{
            delete_error_code, get_error_code, list_error_codes,
            register_error_code, update_error_code_message_and_details,
        },
        webhook::{
            delete_webhook, list_webhooks, register_webhook, update_webhook,
        },
    },
};
use myc_diesel::repositories::SqlAppModule;
use shaku::HasComponent;
use std::str::FromStr;

pub async fn dispatch_system_manager(
    profile: &MyceliumProfileData,
    app_module: &web::Data<SqlAppModule>,
    life_cycle_settings: Option<&web::Data<AccountLifeCycle>>,
    method: &str,
    params: Option<serde_json::Value>,
) -> Result<serde_json::Value, JsonRpcError> {
    match method {
        method_names::SYSTEM_MANAGER_ERROR_CODES_CREATE => {
            let p: RegisterErrorCodeParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let error_code = register_error_code(
                profile.to_profile(),
                p.prefix,
                p.message,
                p.details,
                p.is_internal,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(error_code).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        method_names::SYSTEM_MANAGER_ERROR_CODES_LIST => {
            let p: ListErrorCodesParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let result = list_error_codes(
                profile.to_profile(),
                p.prefix,
                p.code,
                p.is_internal,
                p.page_size,
                p.skip,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        method_names::SYSTEM_MANAGER_ERROR_CODES_GET => {
            let p: GetErrorCodeParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = get_error_code(
                profile.to_profile(),
                p.prefix,
                p.code,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_response_kind_to_result(result)
        }
        method_names::SYSTEM_MANAGER_ERROR_CODES_UPDATE_MESSAGE_AND_DETAILS => {
            let p: UpdateErrorCodeMessageAndDetailsParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let error_code = update_error_code_message_and_details(
                profile.to_profile(),
                p.prefix,
                p.code,
                p.message,
                p.details,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            serde_json::to_value(error_code).map_err(|e| JsonRpcError {
                code: types::codes::INTERNAL_ERROR,
                message: e.to_string(),
                data: None,
            })
        }
        method_names::SYSTEM_MANAGER_ERROR_CODES_DELETE => {
            let p: DeleteErrorCodeParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            delete_error_code(
                profile.to_profile(),
                p.prefix,
                p.code,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            Ok(serde_json::Value::Null)
        }
        method_names::SYSTEM_MANAGER_WEBHOOKS_CREATE => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: RegisterWebhookParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let trigger = WebHookTrigger::from_str(&p.trigger)
                .map_err(|e| invalid_params(e))?;
            let method: Option<HttpMethod> = p
                .method
                .as_ref()
                .map(|s| {
                    serde_json::from_value(serde_json::Value::String(
                        s.to_uppercase(),
                    ))
                    .map_err(|e| invalid_params(e.to_string()))
                })
                .transpose()?;
            if let Some(ref m) = method {
                if !WebHook::is_write_method(m) {
                    return Err(invalid_params(
                        "HTTP method must be POST, PUT, PATCH or DELETE"
                            .to_string(),
                    ));
                }
            }
            let secret: Option<HttpSecret> = p
                .secret
                .map(|v| {
                    serde_json::from_value(v)
                        .map_err(|e| invalid_params(e.to_string()))
                })
                .transpose()?;
            let result = register_webhook(
                profile.to_profile(),
                p.name,
                p.description,
                p.url,
                trigger,
                method,
                secret,
                life_cycle.to_owned(),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            create_response_kind_to_result(result)
        }
        method_names::SYSTEM_MANAGER_WEBHOOKS_LIST => {
            let p: ListWebhooksParams = params
                .map(serde_json::from_value)
                .transpose()
                .map_err(|e| invalid_params(e.to_string()))?
                .unwrap_or_default();
            let trigger = p
                .trigger
                .as_ref()
                .map(|s| {
                    WebHookTrigger::from_str(s).map_err(|e| invalid_params(e))
                })
                .transpose()?;
            let result = list_webhooks(
                profile.to_profile(),
                p.name,
                trigger,
                p.page_size,
                p.skip,
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            fetch_many_response_kind_to_result(result)
        }
        method_names::SYSTEM_MANAGER_WEBHOOKS_UPDATE => {
            let life_cycle = life_cycle_settings
                .ok_or_else(|| invalid_params("Life cycle config required"))?
                .get_ref();
            let p: UpdateWebhookParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let secret: Option<HttpSecret> = p
                .secret
                .map(|v| {
                    serde_json::from_value(v)
                        .map_err(|e| invalid_params(e.to_string()))
                })
                .transpose()?;
            let result = update_webhook(
                profile.to_profile(),
                p.webhook_id,
                p.name,
                p.description,
                secret,
                life_cycle.to_owned(),
                p.is_active,
                Box::new(&*app_module.resolve_ref()),
                Box::new(&*app_module.resolve_ref()),
            )
            .await
            .map_err(mapped_errors_to_jsonrpc_error)?;
            updating_response_kind_to_result(result)
        }
        method_names::SYSTEM_MANAGER_WEBHOOKS_DELETE => {
            let p: DeleteWebhookParams =
                serde_json::from_value(params.ok_or_else(params_required)?)
                    .map_err(|e| invalid_params(e.to_string()))?;
            let result = delete_webhook(
                profile.to_profile(),
                p.webhook_id,
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
