use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
use myc_http_tools::{
    dtos::gateway_profile_data::GatewayProfileData,
    settings::{DEFAULT_PROFILE_KEY, DEFAULT_PROFILE_SIGNATURE_KEY},
};
use serde_json::json;
use std::sync::atomic::{AtomicU64, Ordering};

const CONTROL_HEADER: &str = "x-psig-control";

/// How many times the protected handler actually ran
#[derive(Default)]
pub(crate) struct WhoamiCalls(AtomicU64);

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config
        .service(health)
        .service(openapi)
        .service(public)
        .service(authenticated)
        .service(capture)
        .service(stats)
        .service(whoami);
}

// ? ---------------------------------------------------------------------------
// ? Routes without the SDK
// ? ---------------------------------------------------------------------------

#[get("/health")]
async fn health() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "status": "ok",
        "runId": std::env::var("PSIG_RUN_ID").ok(),
        "verification": std::env::var("MYC_PROFILE_PUBLIC_KEY_1").is_ok(),
    }))
}

#[get("/openapi.json")]
async fn openapi() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "openapi": "3.0.3",
        "info": { "title": "PSIG synthetic consumer", "version": "1" },
        "paths": { "/whoami": { "get": {
            "operationId": "psigWhoami",
            "responses": { "200": { "description": "Account" } },
        } } },
    }))
}

#[get("/public")]
async fn public(req: HttpRequest) -> impl Responder {
    inspect(&req)
}

#[get("/authenticated")]
async fn authenticated(req: HttpRequest) -> impl Responder {
    inspect(&req)
}

/// Echo the identity headers, only with the private control token
#[get("/capture")]
async fn capture(req: HttpRequest) -> impl Responder {
    if !allowed(&req) {
        return HttpResponse::Forbidden().finish();
    }

    HttpResponse::Ok().json(json!({
        "profile": header_text(&req, DEFAULT_PROFILE_KEY),
        "signature": header_text(&req, DEFAULT_PROFILE_SIGNATURE_KEY),
    }))
}

#[get("/stats")]
async fn stats(calls: web::Data<WhoamiCalls>) -> impl Responder {
    HttpResponse::Ok().json(json!({ "whoami": calls.0.load(Ordering::SeqCst) }))
}

// ? ---------------------------------------------------------------------------
// ? Protected route: the SDK extractor runs before the handler
// ? ---------------------------------------------------------------------------

#[get("/whoami")]
async fn whoami(
    profile: GatewayProfileData,
    calls: web::Data<WhoamiCalls>,
) -> impl Responder {
    calls.0.fetch_add(1, Ordering::SeqCst);

    HttpResponse::Ok().json(json!({
        "framework": "actix-web",
        "accId": profile.acc_id.to_string(),
    }))
}

// ? ---------------------------------------------------------------------------
// ? Helpers
// ? ---------------------------------------------------------------------------

fn inspect(req: &HttpRequest) -> HttpResponse {
    let headers = req.headers();

    HttpResponse::Ok().json(json!({
        "profilePresent": headers.contains_key(DEFAULT_PROFILE_KEY),
        "signaturePresent": headers.contains_key(DEFAULT_PROFILE_SIGNATURE_KEY),
    }))
}

fn header_text(req: &HttpRequest, name: &str) -> Option<String> {
    req.headers().get(name)?.to_str().ok().map(str::to_string)
}

fn allowed(req: &HttpRequest) -> bool {
    let expected = std::env::var("PSIG_CONTROL_TOKEN").unwrap_or_default();
    let actual = header_text(req, CONTROL_HEADER).unwrap_or_default();

    !expected.is_empty() && same_bytes(actual.as_bytes(), expected.as_bytes())
}

/// Constant-time comparison (no early exit on the first different byte)
fn same_bytes(left: &[u8], right: &[u8]) -> bool {
    let difference = left
        .iter()
        .zip(right)
        .fold(0u8, |accumulator, (one, other)| accumulator | (one ^ other));

    left.len() == right.len() && difference == 0
}
