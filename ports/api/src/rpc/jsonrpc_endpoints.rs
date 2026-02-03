use super::handlers::admin_jsonrpc_post;
use crate::rpc::openrpc::{generate_openrpc_spec, OpenRpcSpecConfig};

use actix_web::{route, web, HttpResponse, Responder};

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(admin_jsonrpc_post).service(openrpc_spec);
}

#[route("", method = "GET", method = "POST")]
pub async fn openrpc_spec(
    config: web::Data<OpenRpcSpecConfig>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .json(generate_openrpc_spec(config.get_ref()))
}
