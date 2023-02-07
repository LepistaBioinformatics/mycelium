use actix_web::{get, web, App, HttpResponse, HttpServer, Responder};
use std::env::var_os;

#[get("/")]
async fn default_public() -> impl Responder {
    HttpResponse::Ok().body("success")
}

#[get("/")]
async fn protected() -> impl Responder {
    HttpResponse::Ok().body("success")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| {
        App::new()
            .service(web::scope("/public").service(default_public))
            .service(web::scope("/protected").service(protected))
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
