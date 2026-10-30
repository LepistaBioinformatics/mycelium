//! Synthetic actix-web downstream for the profile signature E2E (test only).
//!
//! Uses the LOCAL `mycelium-http-tools` crate (path dependency) and its
//! `GatewayProfileData` extractor, never a published crate. The verification
//! settings (`MYC_PROFILE_PUBLIC_KEY_*`, `MYC_PROFILE_SIGNATURE_AUDIENCE`) are
//! loaded once at startup; an invalid setting stops the service.
//!
//! Run: `psig-rust-consumer <port>`

mod routes;

use actix_web::{web, App, HttpServer};
use myc_http_tools::models::ProfileSignatureVerification;
use routes::{configure, WhoamiCalls};
use std::io::{Error, ErrorKind, Result};

#[actix_web::main]
async fn main() -> Result<()> {
    let port = listen_port()?;
    let verification = web::Data::new(load_verification()?);
    let calls = web::Data::new(WhoamiCalls::default());

    HttpServer::new(move || {
        App::new()
            .app_data(verification.clone())
            .app_data(calls.clone())
            .configure(configure)
    })
    .workers(1)
    .bind(("127.0.0.1", port))?
    .run()
    .await
}

fn listen_port() -> Result<u16> {
    std::env::args()
        .nth(1)
        .and_then(|raw| raw.parse().ok())
        .ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "usage: psig-rust-consumer <port>",
            )
        })
}

fn load_verification() -> Result<ProfileSignatureVerification> {
    ProfileSignatureVerification::from_environment()
        .map_err(|error| Error::new(ErrorKind::InvalidInput, error.msg()))
}
