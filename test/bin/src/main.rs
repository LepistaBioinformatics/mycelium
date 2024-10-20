use actix_web::{get, post, web, App, HttpResponse, HttpServer, Responder};
use myc_http_tools::Account;
use serde::Deserialize;
use std::env::var_os;

#[get("/")]
async fn default_public() -> impl Responder {
    HttpResponse::Ok().body("success")
}

#[get("/")]
async fn protected() -> impl Responder {
    HttpResponse::Ok().body("success")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebHookBody {
    pub account: Account,
}

#[post("/")]
async fn webhook(body: web::Json<WebHookBody>) -> impl Responder {
    println!("{:?}", body);

    HttpResponse::Ok().body("success")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::scope("/public").service(default_public))
            .service(web::scope("/protected").service(protected))
            .service(web::scope("/webhook").service(webhook))
    })
    .bind((
        "127.0.0.1",
        match var_os("SERVICE_PORT") {
            Some(path) => path.into_string().unwrap().parse::<u16>().unwrap(),
            None => 8080,
        },
    ))?
    .run()
    .await
}
