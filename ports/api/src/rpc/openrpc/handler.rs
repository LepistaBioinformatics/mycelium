use super::config::OpenRpcSpecConfig;
use super::spec;

use actix_web::{get, web, HttpResponse, Responder};

#[get("")]
pub async fn openrpc_spec(
    config: web::Data<OpenRpcSpecConfig>,
) -> impl Responder {
    HttpResponse::Ok()
        .content_type("application/json")
        .json(spec::generate_openrpc_spec(config.get_ref()))
}
