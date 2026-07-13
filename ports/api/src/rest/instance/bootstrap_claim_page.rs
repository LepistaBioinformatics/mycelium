use crate::models::active_backend_modules::SqlAppModule;

use actix_web::{get, web, HttpResponse, Responder};
use myc_core::{
    domain::entities::InstanceSettingsFetching, models::AccountLifeCycle,
    settings::TEMPLATES,
    use_cases::super_users::staff::bootstrap::staff_bootstrap_is_pending,
};
use shaku::HasComponent;
use tera::Context as TeraContext;
use tracing::warn;

/// Serve the one-time staff bootstrap claim page.
///
/// Public (no auth). Returns 404 unconditionally when the feature is
/// disabled (`staff_bootstrap_secret` unset) or already claimed -- the same
/// rendered error page for every cause, so nothing about *why* is leaked
/// beyond "not available" (spec SB-R4/SB-R8).
#[utoipa::path(
    get,
    operation_id = "staff_bootstrap_claim_page",
    context_path = "/instance",
    responses(
        (
            status = 200,
            description = "Bootstrap claim form.",
            content_type = "text/html",
        ),
        (
            status = 404,
            description = "Bootstrap disabled or already claimed.",
            content_type = "text/html",
        ),
    ),
    security(()),
)]
#[get("/bootstrap")]
pub async fn bootstrap_claim_page_url(
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let domain_name = life_cycle_settings
        .domain_name
        .async_get_or_error()
        .await
        .unwrap_or_default();

    if life_cycle_settings.staff_bootstrap_secret.is_none() {
        return render_bootstrap_error_page(&domain_name);
    }

    let settings_repo: &dyn InstanceSettingsFetching =
        &*sql_app_module.resolve_ref();

    match staff_bootstrap_is_pending(Box::new(settings_repo)).await {
        Ok(true) => {}
        _ => return render_bootstrap_error_page(&domain_name),
    }

    let mut context = TeraContext::new();
    context.insert("domain_name", &domain_name);

    match TEMPLATES.render("web/instance-bootstrap-claim.html", &context) {
        Ok(html) => HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(err) => {
            warn!(
                "Failed to render instance-bootstrap-claim template: {}",
                err
            );
            HttpResponse::InternalServerError().finish()
        }
    }
}

fn render_bootstrap_error_page(domain_name: &str) -> HttpResponse {
    let mut context = TeraContext::new();
    context.insert("domain_name", domain_name);

    match TEMPLATES.render("web/instance-bootstrap-error.html", &context) {
        Ok(html) => HttpResponse::NotFound()
            .content_type("text/html; charset=utf-8")
            .body(html),
        Err(err) => {
            warn!(
                "Failed to render instance-bootstrap-error template: {}",
                err
            );
            HttpResponse::NotFound().finish()
        }
    }
}
