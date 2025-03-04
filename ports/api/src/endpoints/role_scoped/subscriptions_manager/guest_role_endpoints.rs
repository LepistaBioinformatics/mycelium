use crate::{dtos::MyceliumProfileData, endpoints::shared::PaginationParams};

use actix_web::{get, web, Responder};
use myc_core::{
    domain::dtos::guest_role::GuestRole,
    use_cases::role_scoped::subscriptions_manager::guest_role::list_guest_roles,
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        fetch_many_response_kind, handle_mapped_error,
    },
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::{IntoParams, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(list_guest_roles_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListGuestRolesParams {
    pub name: Option<String>,
}

/// List Roles
#[utoipa::path(
    get,
    params(
        ListGuestRolesParams,
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Not found.",
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = HttpJsonResponse,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = HttpJsonResponse,
        ),
        (
            status = 200,
            description = "Success.",
            body = [GuestRole],
        ),
    ),
)]
#[get("")]
pub async fn list_guest_roles_url(
    info: web::Query<ListGuestRolesParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    match list_guest_roles(
        profile.to_profile(),
        info.name.to_owned(),
        page.page_size,
        page.skip,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => fetch_many_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
