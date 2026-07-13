use crate::{
    models::active_backend_modules::SqlAppModule,
    rest::role_scoped::beginners::user_endpoints::MyceliumLoginResponse,
};

use actix_web::{post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::email::Email,
        entities::{
            AccountRegistration, InstanceSettingsFetching,
            InstanceSettingsRegistration, TokenInvalidation, UserFetching,
            UserRegistration, UserUpdating,
        },
    },
    models::AccountLifeCycle,
    use_cases::{
        role_scoped::beginner::user::verify_magic_link,
        super_users::staff::bootstrap::{
            claim_staff_bootstrap, validate_bootstrap_secret,
        },
    },
};
use myc_http_tools::{
    functions::encode_jwt, models::internal_auth_config::InternalOauthConfig,
    wrappers::default_response_to_http_response::handle_mapped_error,
};
use serde::Deserialize;
use shaku::HasComponent;
use utoipa::ToSchema;

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct BootstrapCompleteBody {
    secret: String,
    email: String,
    code: String,
}

/// Complete the staff bootstrap: verify the magic-link code and claim the
/// initial Staff account.
///
/// Public (no auth). Delegates entirely to the existing, unmodified
/// `verify_magic_link` use-case for authentication, then -- only on success
/// -- calls `claim_staff_bootstrap` (CAS-first, design.md D-1). Returns the
/// same `MyceliumLoginResponse` shape a normal magic-link verify would, so
/// the operator is immediately logged in as the new Staff user.
#[utoipa::path(
    post,
    operation_id = "staff_bootstrap_complete",
    context_path = "/instance",
    request_body = BootstrapCompleteBody,
    responses(
        (
            status = 200,
            description = "Staff account claimed; session issued.",
            body = MyceliumLoginResponse,
        ),
        (
            status = 401,
            description = "Invalid secret, invalid/expired code, or already claimed.",
        ),
    ),
    security(()),
)]
#[post("/bootstrap/complete")]
pub async fn bootstrap_complete_url(
    body: web::Json<BootstrapCompleteBody>,
    app_module: web::Data<SqlAppModule>,
    auth_config: web::Data<InternalOauthConfig>,
    core_config: web::Data<AccountLifeCycle>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Ok(email) => email,
        Err(_) => return HttpResponse::Unauthorized().finish(),
    };

    let configured_secret = match &core_config.staff_bootstrap_secret {
        Some(resolver) => resolver.async_get_or_error().await.ok(),
        None => None,
    };

    if let Err(err) = validate_bootstrap_secret(
        &body.secret,
        configured_secret.as_deref(),
        Box::new(&*app_module.resolve_ref() as &dyn InstanceSettingsFetching),
    )
    .await
    {
        return handle_mapped_error(err);
    }

    let user = match verify_magic_link(
        email,
        body.code.to_owned(),
        Box::new(&*app_module.resolve_ref() as &dyn UserFetching),
        Box::new(&*app_module.resolve_ref() as &dyn UserRegistration),
        Box::new(&*app_module.resolve_ref() as &dyn UserUpdating),
        Box::new(&*app_module.resolve_ref() as &dyn TokenInvalidation),
    )
    .await
    {
        Ok(user) => user,
        Err(err) => return handle_mapped_error(err),
    };

    if let Err(err) = claim_staff_bootstrap(
        user.to_owned(),
        Box::new(&*app_module.resolve_ref() as &dyn AccountRegistration),
        Box::new(
            &*app_module.resolve_ref() as &dyn InstanceSettingsRegistration
        ),
    )
    .await
    {
        return handle_mapped_error(err);
    };

    match encode_jwt(
        user.to_owned(),
        auth_config.get_ref().to_owned(),
        core_config.get_ref().to_owned(),
        false,
    )
    .await
    {
        Err(err) => err,
        Ok((token, duration)) => {
            HttpResponse::Ok().json(MyceliumLoginResponse {
                token,
                duration,
                totp_required: false,
                user,
            })
        }
    }
}
