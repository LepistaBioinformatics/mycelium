use actix_web::{
    get, post, web, App, HttpRequest, HttpResponse, HttpServer, Responder,
};
use myc_http_tools::dtos::gateway_profile_data::GatewayProfileData;
use serde::Deserialize;
use std::env::var_os;

#[get("")]
async fn health() -> impl Responder {
    HttpResponse::Ok().body("success")
}

// ? ---------------------------------------------------------------------------
// ? Public Route
// ? ---------------------------------------------------------------------------

#[get("")]
async fn public() -> impl Responder {
    HttpResponse::Ok().body("success")
}

// ? ---------------------------------------------------------------------------
// ? Protected Route
// ? ---------------------------------------------------------------------------

#[get("")]
async fn protected(profile: GatewayProfileData) -> impl Responder {
    println!("{:?}", profile);

    HttpResponse::Ok().body("success")
}

// ? ---------------------------------------------------------------------------
// ? Expects Header Route
// ? ---------------------------------------------------------------------------

#[get("")]
async fn expects_header(req: HttpRequest) -> impl Responder {
    println!("Headers: {:?}", req.headers());

    HttpResponse::Ok().body("success")
}

// ? ---------------------------------------------------------------------------
// ? Webhook Route
// ? ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebHookBody {
    pub id: String,
    pub name: String,
    pub created: String,
}

#[post("")]
async fn webhook(body: web::Json<WebHookBody>) -> impl Responder {
    println!("{:?}", body);

    HttpResponse::Ok().body("success")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let address = (
        "127.0.0.1",
        match var_os("SERVICE_PORT") {
            Some(path) => path.into_string().unwrap().parse::<u16>().unwrap(),
            None => 8080,
        },
    );

    HttpServer::new(|| {
        App::new()
            .service(web::scope("/health").service(health))
            .service(web::scope("/public").service(public))
            .service(web::scope("/protected").service(protected))
            .service(web::scope("/protected-by-roles").service(protected))
            .service(
                web::scope("/protected-by-permissioned-roles")
                    .service(protected),
            )
            .service(
                web::scope("/protected-by-service-token-with-role")
                    .service(protected),
            )
            .service(
                web::scope(
                    "/protected-by-service-token-with-permissioned-roles",
                )
                .service(protected),
            )
            .service(web::scope("/webhook").service(webhook))
            .service(web::scope("/expects-header").service(expects_header))
    })
    .bind(address)?
    .run()
    .await
}
