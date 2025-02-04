use crate::{
    endpoints::shared::{build_actor_context, UrlGroup},
    middleware::{
        check_credentials_with_multi_identity_provider,
        parse_issuer_from_request,
    },
};

use actix_web::{head, post, web, HttpRequest, HttpResponse, Responder};
use chrono::Duration;
use myc_core::{
    domain::{
        actors::SystemActor,
        dtos::user::{Provider, Totp, User},
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::user::{
        check_email_password_validity, check_email_registration_status,
        check_token_and_activate_user, check_token_and_reset_password,
        create_default_user, start_password_redefinition, totp_check_token,
        totp_disable, totp_finish_activation, totp_start_activation,
        EmailRegistrationStatus,
    },
};
use myc_diesel::repositories::SqlAppModule;
use myc_http_tools::{
    functions::encode_jwt, models::internal_auth_config::InternalOauthConfig,
    responses::GatewayError, utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::handle_mapped_error, Email,
};
use myc_notifier::repositories::NotifierAppModule;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::json;
use shaku::HasComponent;
use tracing::{error, warn};
use utoipa::{IntoParams, ToResponse, ToSchema};

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(check_email_registration_status_url)
        .service(create_default_user_url)
        .service(check_user_token_url)
        .service(start_password_redefinition_url)
        .service(check_token_and_reset_password_url)
        .service(check_email_password_validity_url)
        .service(totp_start_activation_url)
        .service(totp_finish_activation_url)
        .service(totp_check_token_url)
        .service(totp_disable_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckEmailStatusQuery {
    email: String,
}

fn serialize_duration<S>(
    duration: &Duration,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_u64(duration.num_seconds() as u64)
}

#[derive(Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MyceliumLoginResponse {
    token: String,
    #[serde(serialize_with = "serialize_duration")]
    duration: Duration,
    totp_required: bool,

    #[serde(flatten)]
    user: User,
}

#[derive(Deserialize, ToSchema, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct TotpActivationStartedParams {
    qr_code: Option<bool>,
}

#[derive(Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpActivationStartedResponse {
    totp_url: Option<String>,
}

#[derive(Serialize, ToResponse, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpActivationFinishedResponse {
    finished: bool,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TotpUpdatingValidationBody {
    token: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultUserBody {
    email: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckTokenBody {
    token: String,
    email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StartPasswordResetBody {
    email: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResetPasswordBody {
    token: String,
    email: String,
    new_password: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CheckUserCredentialsBody {
    email: String,
    password: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// User
//
// ? ---------------------------------------------------------------------------

/// Check email status
///
/// Check if the email is already registered.
///
#[utoipa::path(
    head,
    params(
        (
            "email" = String,
            Query,
            description = "The email to be checked.",
        )
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            status = 400,
            description = "Bad request.",
            body = HttpJsonResponse,
        ),
        (
            status = 204,
            description = "Success.",
        ),
    ),
    security(()),
)]
#[head("/status")]
pub async fn check_email_registration_status_url(
    query: web::Query<CheckEmailStatusQuery>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email_instance = match Email::from_string(query.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {err}");
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(
                    "Invalid email address.".to_string(),
                ),
            );
        }
        Ok(email) => email,
    };

    match check_email_registration_status(
        email_instance,
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => {
            let mut response = HttpResponse::NoContent();
            let status_header = "X-Email-Registration-Status";
            let account_creation_header = "X-Account-Created";
            let provider_header = "X-Email-Provider";

            match res {
                EmailRegistrationStatus::NotRegistered(_) => {
                    response.append_header((status_header, "NotRegistered"));
                }
                EmailRegistrationStatus::WaitingActivation(_) => {
                    response
                        .append_header((status_header, "WaitingActivation"));
                }
                EmailRegistrationStatus::RegisteredWithInternalProvider(
                    provider,
                ) => {
                    response.append_header((
                        status_header,
                        "RegisteredWithInternalProvider",
                    ));

                    response.append_header((
                        account_creation_header,
                        provider.account_created.to_string(),
                    ));

                    if let Some(provider) = provider.provider {
                        response.append_header((
                            provider_header,
                            match provider {
                                Provider::Internal(_) => "Internal".to_string(),
                                _ => "Internal".to_string(),
                            },
                        ));
                    }
                }
                EmailRegistrationStatus::RegisteredWithExternalProvider(
                    provider,
                ) => {
                    response.append_header((
                        status_header,
                        "RegisteredWithExternalProvider",
                    ));

                    response.append_header((
                        account_creation_header,
                        provider.account_created.to_string(),
                    ));

                    if let Some(provider) = provider.provider {
                        response.append_header((
                            provider_header,
                            match provider {
                                Provider::External(res) => res.to_string(),
                                _ => "External".to_string(),
                            },
                        ));
                    }
                }
            }

            response.finish()
        }
        Err(err) => {
            error!("Error checking email registration status: {err}");

            handle_mapped_error(err)
        }
    }
}

/// Register user
///
/// This route should be used to register a new user. If the Bearer token is
/// included in the request, the user will be registered with the provider
/// informed in the token. Otherwise, the password is required.
///
#[utoipa::path(
    post,
    params(
        (
            "Authorization" = Option<String>,
            Header,
            description = "An optional Bearer token. When included, the user \
            will be registered with the provider informed in the token.",
        )
    ),
    request_body = CreateDefaultUserBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            status = 201,
            description = "User successfully created.",
            body = User,
        ),
    ),
    security(()),
)]
#[post("")]
pub async fn create_default_user_url(
    req: HttpRequest,
    body: web::Json<CreateDefaultUserBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
    notifier_module: web::Data<NotifierAppModule>,
) -> impl Responder {
    let provider = match parse_issuer_from_request(req.clone()).await {
        Err(err) => match err {
            GatewayError::Unauthorized(_) => None,
            _ => {
                warn!("Invalid issuer: {err}");

                return HttpResponse::BadRequest().json(
                    HttpJsonResponse::new_message(
                        "Invalid issuer.".to_string(),
                    ),
                );
            }
        },
        Ok((issuer, _)) => Some(issuer),
    };

    match create_default_user(
        body.email.to_owned(),
        body.first_name.to_owned(),
        body.last_name.to_owned(),
        body.password.to_owned(),
        provider,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*notifier_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => HttpResponse::Created().json(json!({"id": res})),
        Err(err) => handle_mapped_error(err),
    }
}

/// Check token and activate user
///
/// This route should be used to check the token and activate the user.
///
#[utoipa::path(
    post,
    request_body = CheckTokenBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Activation token is valid.",
            body = bool,
        ),
    ),
    security(()),
)]
#[post("/validate-activation-token")]
pub async fn check_user_token_url(
    body: web::Json<CheckTokenBody>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(
                    "Invalid email address.".to_string(),
                ),
            );
        }
        Ok(email) => email,
    };

    match check_token_and_activate_user(
        body.token.to_owned(),
        email,
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().finish(),
        Err(err) => handle_mapped_error(err),
    }
}

/// Start password redefinition
///
/// This route should be used to start the password redefinition process.
///
#[utoipa::path(
    post,
    request_body = StartPasswordResetBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Password change requested.",
            body = bool,
        ),
    ),
    security(()),
)]
#[post("/start-password-reset")]
pub async fn start_password_redefinition_url(
    body: web::Json<StartPasswordResetBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
    notifier_module: web::Data<NotifierAppModule>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(
                    "Invalid email address.".to_string(),
                ),
            );
        }
        Ok(email) => email,
    };

    match start_password_redefinition(
        email,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*notifier_module.resolve_ref()),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().json(true),
        Err(err) => handle_mapped_error(err),
    }
}

/// Check token and reset password
///
/// This route should be used to check the token and reset the password.
///
#[utoipa::path(
    post,
    request_body = ResetPasswordBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Password change requested.",
            body = bool,
        ),
    ),
    security(()),
)]
#[post("/reset-password")]
pub async fn check_token_and_reset_password_url(
    body: web::Json<ResetPasswordBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
    notifier_module: web::Data<NotifierAppModule>,
) -> impl Responder {
    let email = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(
                    "Invalid email address.".to_string(),
                ),
            );
        }
        Ok(email) => email,
    };

    match check_token_and_reset_password(
        body.token.to_owned(),
        email,
        body.new_password.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*notifier_module.resolve_ref()),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok().json(true),
        Err(err) => handle_mapped_error(err),
    }
}

/// Login with email and password
///
/// This route should be used to login with email and password. If the user has
/// enabled the TOTP app, the user will be redirected to the TOTP activation
/// route.
///
#[utoipa::path(
    post,
    request_body = CheckUserCredentialsBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Credentials are valid.",
            body = MyceliumLoginResponse,
        ),
    ),
    security(()),
)]
#[post("/login")]
pub async fn check_email_password_validity_url(
    body: web::Json<CheckUserCredentialsBody>,
    app_module: web::Data<SqlAppModule>,
    auth_config: web::Data<InternalOauthConfig>,
) -> impl Responder {
    let email_instance = match Email::from_string(body.email.to_owned()) {
        Err(err) => {
            warn!("Invalid email: {}", err);
            return HttpResponse::BadRequest().json(
                HttpJsonResponse::new_message(
                    "Invalid email address.".to_string(),
                ),
            );
        }
        Ok(email) => email,
    };

    match check_email_password_validity(
        email_instance,
        body.password.to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Err(err) => handle_mapped_error(err),
        Ok((valid, user)) => match valid {
            true => {
                let _user = if let Some(user) = user {
                    user
                } else {
                    return HttpResponse::NoContent().finish();
                };

                match _user.mfa().totp {
                    //
                    // If TOTP is disabled, we can proceed with the login
                    // process without any further checks.
                    //
                    Totp::Disabled | Totp::Unknown => match encode_jwt(
                        _user.to_owned(),
                        auth_config.get_ref().to_owned(),
                        false,
                    )
                    .await
                    {
                        Err(err) => return err,
                        Ok((token, duration)) => {
                            return HttpResponse::Ok().json(
                                MyceliumLoginResponse {
                                    token,
                                    duration,
                                    totp_required: false,
                                    user: _user,
                                },
                            )
                        }
                    },
                    //
                    // If TOTP is enabled, we need to check if the user has
                    // already verified the TOTP app.
                    //
                    Totp::Enabled { verified, .. } => {
                        if !verified {
                            //
                            // Redirect user to TOTP activation
                            //
                            return HttpResponse::TemporaryRedirect()
                                .append_header((
                                    "Location",
                                    format!(
                                        "{}/totp/enable",
                                        build_actor_context(
                                            SystemActor::Beginner,
                                            UrlGroup::Users
                                        )
                                    ),
                                ))
                                .finish();
                        }

                        match encode_jwt(
                            _user.to_owned(),
                            auth_config.get_ref().to_owned(),
                            true,
                        )
                        .await
                        {
                            Err(err) => return err,
                            Ok((token, duration)) => {
                                return HttpResponse::Ok().json(
                                    MyceliumLoginResponse {
                                        token,
                                        duration,
                                        totp_required: true,
                                        user: _user,
                                    },
                                )
                            }
                        }
                    }
                }
            }
            false => HttpResponse::Unauthorized().finish(),
        },
    }
}

/// Enable TOTP
///
/// This route should be used to enable the TOTP app. Before enabling the TOTP
/// the user must be authenticated using the `/login/` route.
///
#[utoipa::path(
    post,
    params(TotpActivationStartedParams),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Totp Activation Started.",
            body = TotpActivationStartedResponse,
        ),
        (
            status = 200,
            description = "Totp Activation Started.",
            body = String,
        ),
    ),
)]
#[post("/totp/enable")]
pub async fn totp_start_activation_url(
    req: HttpRequest,
    query: web::Query<TotpActivationStartedParams>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
    notifier_module: web::Data<NotifierAppModule>,
) -> impl Responder {
    let email = match check_credentials_with_multi_identity_provider(req).await
    {
        Err(err) => {
            warn!("err: {:?}", err);
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
        Ok(res) => res,
    };

    let as_qr_code = query.qr_code.to_owned().unwrap_or(false);

    match totp_start_activation(
        email,
        query.qr_code,
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*notifier_module.resolve_ref()),
    )
    .await
    {
        Ok((totp_url, qr_code)) => {
            if as_qr_code && qr_code.is_some() {
                return HttpResponse::build(StatusCode::OK)
                    .content_type("image/jpeg")
                    .body(qr_code.unwrap());
            };

            HttpResponse::Ok().json(TotpActivationStartedResponse { totp_url })
        }
        Err(err) => handle_mapped_error(err),
    }
}

/// Validate TOTP app
///
/// This route should be used to validate the TOTP app after enabling it.
///
#[utoipa::path(
    post,
    request_body = TotpUpdatingValidationBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Credentials are valid.",
            body = MyceliumLoginResponse,
        ),
    ),
)]
#[post("/totp/validate-app")]
pub async fn totp_finish_activation_url(
    req: HttpRequest,
    body: web::Json<TotpUpdatingValidationBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
    notifier_module: web::Data<NotifierAppModule>,
) -> impl Responder {
    let email = match check_credentials_with_multi_identity_provider(req).await
    {
        Err(err) => {
            warn!("err: {:?}", err);
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
        Ok(res) => res,
    };

    match totp_finish_activation(
        email,
        body.token.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*notifier_module.resolve_ref()),
    )
    .await
    {
        Ok(_) => HttpResponse::Ok()
            .json(TotpActivationFinishedResponse { finished: true }),
        Err(err) => handle_mapped_error(err),
    }
}

/// Check TOTP token
///
/// This route should be used to check the TOTP token when tht totp app is
/// enabled.
///
#[utoipa::path(
    post,
    request_body = TotpUpdatingValidationBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Credentials are valid.",
            body = MyceliumLoginResponse,
        ),
    ),
)]
#[post("/totp/check-token")]
pub async fn totp_check_token_url(
    req: HttpRequest,
    body: web::Json<TotpUpdatingValidationBody>,
    auth_config: web::Data<InternalOauthConfig>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    app_module: web::Data<SqlAppModule>,
) -> impl Responder {
    let email = match check_credentials_with_multi_identity_provider(req).await
    {
        Err(err) => {
            warn!("err: {:?}", err);
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
        Ok(res) => res,
    };

    match totp_check_token(
        email,
        body.token.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*app_module.resolve_ref()),
    )
    .await
    {
        Ok(res) => {
            match encode_jwt(
                res.to_owned(),
                auth_config.get_ref().to_owned(),
                false,
            )
            .await
            {
                Err(err) => return err,
                Ok((token, duration)) => {
                    return HttpResponse::Ok().json(MyceliumLoginResponse {
                        token,
                        duration,
                        totp_required: false,
                        user: res,
                    })
                }
            }
        }
        Err(err) => handle_mapped_error(err),
    }
}

/// Disable TOTP
///
/// This route should be used to disable the TOTP app.
///
#[utoipa::path(
    post,
    request_body = TotpUpdatingValidationBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = HttpJsonResponse,
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
            description = "Credentials are valid.",
            body = MyceliumLoginResponse,
        ),
    ),
)]
#[post("/totp/disable")]
pub async fn totp_disable_url(
    req: HttpRequest,
    body: web::Json<TotpUpdatingValidationBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    sql_app_module: web::Data<SqlAppModule>,
    notifier_module: web::Data<NotifierAppModule>,
) -> impl Responder {
    let email = match check_credentials_with_multi_identity_provider(req).await
    {
        Err(err) => {
            warn!("err: {:?}", err);
            return HttpResponse::InternalServerError()
                .json(HttpJsonResponse::new_message(err));
        }
        Ok(res) => res,
    };

    match totp_disable(
        email,
        body.token.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*sql_app_module.resolve_ref()),
        Box::new(&*notifier_module.resolve_ref()),
    )
    .await
    {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(err) => handle_mapped_error(err),
    }
}
