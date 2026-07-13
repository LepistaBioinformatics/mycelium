use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::email::Email,
        entities::{
            InstanceSettingsFetching, LocalMessageWrite, TenantFetching,
            TokenRegistration,
        },
    },
    models::AccountLifeCycle,
    use_cases::{
        role_scoped::beginner::user::request_magic_link,
        super_users::staff::bootstrap::validate_bootstrap_secret,
    },
};
use serde::{Deserialize, Serialize};
use shaku::HasComponent;
use tracing::warn;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRequestCodeBody {
    secret: String,
    email: String,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapRequestCodeResponse {
    sent: bool,
}

/// Request a login code for the staff bootstrap flow.
///
/// Public (no auth). Validates the bootstrap secret and delegates entirely
/// to the existing, unmodified `request_magic_link` use-case -- the email
/// still points at the standard magic-link display page (design.md §6).
///
/// Always responds `{ sent: true }`, whether the secret was valid or not,
/// so an outside observer can't distinguish "wrong secret" from "email
/// dispatched" (same enumeration-avoidance posture as magic-link itself).
#[utoipa::path(
    post,
    operation_id = "staff_bootstrap_request_code",
    context_path = "/instance",
    request_body = BootstrapRequestCodeBody,
    responses(
        (
            status = 200,
            description = "Request accepted (regardless of outcome).",
            body = BootstrapRequestCodeResponse,
        ),
    ),
    security(()),
)]
#[post("/bootstrap/request-code")]
pub async fn bootstrap_request_code_url(
    body: web::Json<BootstrapRequestCodeBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(err) => {
            warn!("Invalid email in bootstrap request-code: {}", err);
            return HttpResponse::Ok()
                .json(BootstrapRequestCodeResponse { sent: true });
        }
    };

    let configured_secret = match &life_cycle_settings.staff_bootstrap_secret {
        Some(resolver) => resolver.async_get_or_error().await.ok(),
        None => None,
    };

    let validation = validate_bootstrap_secret(
        &body.secret,
        configured_secret.as_deref(),
        Box::new(
            &*sql_app_module.resolve_ref() as &dyn InstanceSettingsFetching
        ),
    )
    .await;

    if validation.is_err() {
        return HttpResponse::Ok()
            .json(BootstrapRequestCodeResponse { sent: true });
    }

    let display_base_url = match life_cycle_settings.domain_url.clone() {
        Some(resolver) => match resolver.async_get_or_error().await {
            Ok(url) => format!(
                "{}/_adm/beginners/users/magic-link/display",
                url.trim_end_matches('/'),
            ),
            Err(err) => {
                warn!("domain_url resolution failed (suppressed): {:?}", err);
                return HttpResponse::Ok()
                    .json(BootstrapRequestCodeResponse { sent: true });
            }
        },
        None => {
            warn!(
                "domain_url not configured; suppressing bootstrap request-code"
            );
            return HttpResponse::Ok()
                .json(BootstrapRequestCodeResponse { sent: true });
        }
    };

    if let Err(err) = request_magic_link(
        email,
        display_base_url,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref() as &dyn TokenRegistration),
        Box::new(&*sql_app_module.resolve_ref() as &dyn LocalMessageWrite),
        Box::new(&*sql_app_module.resolve_ref() as &dyn TenantFetching),
    )
    .await
    {
        warn!("request_magic_link error (suppressed): {:?}", err);
    }

    HttpResponse::Ok().json(BootstrapRequestCodeResponse { sent: true })
}
