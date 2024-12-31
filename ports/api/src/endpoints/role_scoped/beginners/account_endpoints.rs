use crate::{
    dtos::MyceliumProfileData,
    middleware::check_credentials_with_multi_identity_provider,
    modules::{
        AccountRegistrationModule, AccountUpdatingModule, UserFetchingModule,
        WebHookFetchingModule,
    },
};

use actix_web::{patch, post, web, HttpRequest, HttpResponse, Responder};
use myc_core::{
    domain::entities::{
        AccountRegistration, AccountUpdating, UserFetching, WebHookFetching,
    },
    models::AccountLifeCycle,
    use_cases::role_scoped::beginner::account::{
        create_default_account, update_own_account_name,
    },
};
use myc_http_tools::{
    utils::HttpJsonResponse,
    wrappers::default_response_to_http_response::{
        handle_mapped_error, updating_response_kind,
    },
    Account,
};
use serde::Deserialize;
use shaku_actix::Inject;
use tracing::warn;
use utoipa::ToSchema;
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_default_account_url)
        .service(update_own_account_name_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultAccountBody {
    name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOwnAccountNameAccountBody {
    name: String,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Create a user related account
///
/// A user related account is an account that is created for a physical person.
///
#[utoipa::path(
    post,
    request_body = CreateDefaultAccountBody,
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
            description = "Account successfully created.",
            body = Account,
        ),
        (
            status = 200,
            description = "Account already exists.",
            body = Account,
        ),
    ),
)]
#[post("")]
pub async fn create_default_account_url(
    req: HttpRequest,
    body: web::Json<CreateDefaultAccountBody>,
    life_cycle_settings: web::Data<AccountLifeCycle>,
    user_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    let opt_email =
        match check_credentials_with_multi_identity_provider(req).await {
            Err(err) => {
                warn!("err: {:?}", err);
                return HttpResponse::InternalServerError()
                    .json(HttpJsonResponse::new_message(err.to_string()));
            }
            Ok(res) => res,
        };

    let email = match opt_email {
        None => return HttpResponse::Forbidden()
            .json(HttpJsonResponse::new_message(String::from(
            "Unable o extract user identity from request. Account not created.",
        ))),
        Some(email) => email,
    };

    match create_default_account(
        email,
        body.name.to_owned(),
        life_cycle_settings.get_ref().to_owned(),
        Box::new(&*user_fetching_repo),
        Box::new(&*account_registration_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Ok(res) => HttpResponse::Created().json(res),
        Err(err) => handle_mapped_error(err),
    }
}

/// Update account name
///
/// Update the account name of the account owner.
///
#[utoipa::path(
    patch,
    request_body = UpdateOwnAccountNameAccountBody,
    params(
        ("account_id" = Uuid, Path, description = "The account primary key."),
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
            description = "Account name not updated.",
            body = HttpJsonResponse,
        ),
        (
            status = 202,
            description = "Account name successfully updated.",
            body = Account,
        ),
    ),
)]
#[patch("/{account_id}/update-account-name")]
pub async fn update_own_account_name_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateOwnAccountNameAccountBody>,
    profile: MyceliumProfileData,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    let profile = profile.to_profile();

    if path.to_owned() != profile.acc_id {
        warn!("No account owner trying to perform account updating.");
        warn!(
            "Account {} trying to update {}",
            profile.acc_id,
            path.to_owned()
        );

        return HttpResponse::Forbidden().json(HttpJsonResponse::new_message(
            String::from(
                "Invalid operation. Operation restricted to account owners.",
            ),
        ));
    }

    match update_own_account_name(
        profile,
        body.name.to_owned(),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Ok(res) => updating_response_kind(res),
        Err(err) => handle_mapped_error(err),
    }
}
