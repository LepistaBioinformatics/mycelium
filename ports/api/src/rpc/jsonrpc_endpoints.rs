use super::handlers::admin_jsonrpc_post;
use super::openrpc;

use actix_web::web;

/// Registers the admin JSON-RPC route and the OpenRPC spec route.
pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(admin_jsonrpc_post)
        .service(openrpc::openrpc_spec);
}
