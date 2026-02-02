use super::params;
use crate::models::api_config::ApiConfig;

use actix_web::{get, web, HttpResponse, Responder};
use myc_config::optional_config::OptionalConfig;
use schemars::schema_for;

const OPENRPC_VERSION: &str = "1.2.6";
const API_VERSION: &str = env!("CARGO_PKG_VERSION");
const RPC_PATH: &str = "_adm/rpc";

const ENV_OPENRPC_DEV_URL: &str = "MYCELIUM_OPENRPC_DEV_URL";
const ENV_OPENRPC_PROD_URL: &str = "MYCELIUM_OPENRPC_PROD_URL";

/// Resolved server URLs for the OpenRPC spec (config + env).
#[derive(Clone, Debug)]
pub struct OpenRpcSpecConfig {
    pub dev_url: String,
    pub prod_url: Option<String>,
}

impl Default for OpenRpcSpecConfig {
    fn default() -> Self {
        Self {
            dev_url: format!("http://localhost:8080/{}", RPC_PATH),
            prod_url: None,
        }
    }
}

impl OpenRpcSpecConfig {
    /// Build from ApiConfig; env vars override config file values.
    pub fn from_api_config(api: &ApiConfig) -> Self {
        let dev_url = std::env::var(ENV_OPENRPC_DEV_URL)
            .ok()
            .or_else(|| api.openrpc_dev_url.clone())
            .unwrap_or_else(|| {
                let scheme = if matches!(api.tls, OptionalConfig::Enabled(_)) {
                    "https"
                } else {
                    "http"
                };
                format!(
                    "{}://{}:{}/{}",
                    scheme, api.service_ip, api.service_port, RPC_PATH
                )
            });

        let prod_url = std::env::var(ENV_OPENRPC_PROD_URL)
            .ok()
            .or_else(|| api.openrpc_prod_url.clone());

        Self { dev_url, prod_url }
    }
}

fn param_schema_value<T: schemars::JsonSchema>() -> serde_json::Value {
    serde_json::to_value(&schema_for!(T))
        .expect("param schema must serialize to JSON")
}

/// Build the OpenRPC spec document for the admin JSON-RPC API.
pub fn generate_openrpc_spec(config: &OpenRpcSpecConfig) -> serde_json::Value {
    let mut servers = vec![serde_json::json!({
        "name": "Development",
        "url": config.dev_url,
        "summary": "Local or development server"
    })];

    if let Some(ref url) = config.prod_url {
        servers.push(serde_json::json!({
            "name": "Production",
            "url": url,
            "summary": "Production server"
        }));
    }

    let create_system_account_schema =
        param_schema_value::<params::CreateSystemAccountParams>();
    let create_tenant_schema =
        param_schema_value::<params::CreateTenantParams>();
    let list_tenant_schema = param_schema_value::<params::ListTenantParams>();
    let delete_tenant_schema =
        param_schema_value::<params::DeleteTenantParams>();
    let include_tenant_owner_schema =
        param_schema_value::<params::IncludeTenantOwnerParams>();
    let exclude_tenant_owner_schema =
        param_schema_value::<params::ExcludeTenantOwnerParams>();

    let methods = vec![
        serde_json::json!({
            "name": "rpc.discover",
            "summary": "Discover the API",
            "description": "Returns this OpenRPC spec document. No params.",
            "tags": [{ "name": "discovery" }],
            "params": [],
            "result": {
                "name": "openrpc",
                "description": "The OpenRPC specification document",
                "schema": { "type": "object" }
            }
        }),
        serde_json::json!({
            "name": "managers.accounts.createSystemAccount",
            "summary": "Create a system account",
            "description": "Creates a system account (gateway manager, guests manager, or system manager). Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "accounts" }],
            "params": [{ "name": "params", "description": "Creation parameters", "required": true, "schema": create_system_account_schema }],
            "result": { "name": "result", "description": "Created or existing account (GetOrCreateResponseKind)", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.guestRoles.createSystemRoles",
            "summary": "Create system guest roles",
            "description": "Creates all system guest roles (subscriptions, users, account, guest, gateway, system, tenant managers with read/write). Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "guest-roles" }],
            "params": [],
            "result": { "name": "result", "description": "List of guest roles created", "schema": { "type": "array", "items": { "type": "object" } } },
            "errors": [{ "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.createTenant",
            "summary": "Create a tenant",
            "description": "Creates a new tenant with the given owner. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": create_tenant_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.listTenant",
            "summary": "List tenants",
            "description": "Lists tenants with optional filters (name, owner, metadata, tag) and pagination (pageSize, skip).",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": false, "schema": list_tenant_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.deleteTenant",
            "summary": "Delete a tenant",
            "description": "Deletes a tenant by ID. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": delete_tenant_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.includeTenantOwner",
            "summary": "Include a tenant owner",
            "description": "Adds an owner to a tenant. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": include_tenant_owner_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
        serde_json::json!({
            "name": "managers.tenants.excludeTenantOwner",
            "summary": "Exclude a tenant owner",
            "description": "Removes an owner from a tenant. Requires manager privileges.",
            "tags": [{ "name": "managers" }, { "name": "tenants" }],
            "params": [{ "name": "params", "required": true, "schema": exclude_tenant_owner_schema }],
            "result": { "name": "result", "schema": { "type": "object" } },
            "errors": [{ "code": -32602, "message": "Invalid params" }, { "code": -32401, "message": "Forbidden" }]
        }),
    ];

    serde_json::json!({
        "openrpc": OPENRPC_VERSION,
        "info": {
            "title": "Mycelium Admin JSON-RPC API",
            "description": "JSON-RPC 2.0 API for Mycelium admin operations (managers). Supports single request and batch. Scope and permission checks are enforced by use cases.",
            "version": API_VERSION,
            "contact": {
                "name": "Samuel Galvão Elias",
                "url": "https://github.com/sgelias/mycelium"
            }
        },
        "servers": servers,
        "methods": methods,
        "components": { "schemas": {} }
    })
}

/// GET handler for the OpenRPC spec (e.g. _adm/rpc/openrpc.json).
#[get("")]
pub async fn openrpc_spec(
    config: web::Data<OpenRpcSpecConfig>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .json(generate_openrpc_spec(config.get_ref()))
}
