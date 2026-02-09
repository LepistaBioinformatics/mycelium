//! OpenRPC discovery for JSON-RPC endpoint.
//!
//! Generates an OpenRPC 1.2.x spec describing all methods at `_adm/rpc`.
//! Served at GET `_adm/rpc/openrpc.json` and via the JSON-RPC method `rpc.discover`.
//! Server URLs come from config ([api] openrpcDevUrl / openrpcProdUrl) or env
//! (MYCELIUM_OPENRPC_DEV_URL / MYCELIUM_OPENRPC_PROD_URL).
//! Param schemas are generated from DTOs via schemars.
//!
//! Layout:
//! - [config](config): OpenRpcSpecConfig, server URLs from ApiConfig/env
//! - [schema](schema): param_schema_value for JSON Schema from types
//! - [methods](methods): method descriptors by scope (discovery, managers, beginners)
//! - [spec](spec): assembly of full OpenRPC document
//! - [handler](handler): GET handler for openrpc.json

pub(crate) mod config;
pub(crate) mod methods;
pub(crate) mod schema;
pub(crate) mod spec;

pub use config::OpenRpcSpecConfig;
pub use spec::generate_openrpc_spec;
