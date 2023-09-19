use crate::modules::{
    AccountFetchingModule, AccountRegistrationModule,
    AccountTypeRegistrationModule, AccountUpdatingModule,
};

use actix_web::{patch, post, web, HttpResponse, Responder};
use clean_base::entities::{GetOrCreateResponseKind, UpdatingResponseKind};
use log::warn;
use myc_core::{
    domain::entities::{
        AccountFetching, AccountRegistration, AccountTypeRegistration,
        AccountUpdating,
    },
    use_cases::roles::default_users::account::{
        create_default_account, update_own_account_name,
    },
};
use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::ToSchema;
use uuid::Uuid;

// ? -----------------------------------------------------------------------
// ? Configure application
// ? -----------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/accounts")
            .service(create_default_account_url)
            .service(update_own_account_name_url),
    );
}

// ? -----------------------------------------------------------------------
// ? Define API structs
// ? -----------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateDefaultAccountBody {
    email: String,
    account_name: String,
    first_name: Option<String>,
    last_name: Option<String>,
    password: Option<String>,
    provider_name: Option<String>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOwnAccountNameAccountBody {
    name: String,
}

// ? -----------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? -----------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = "/myc/default-users/accounts",
    request_body = CreateDefaultAccountBody,
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
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
#[post("/")]
pub async fn create_default_account_url(
    body: web::Json<CreateDefaultAccountBody>,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
) -> impl Responder {
    match create_default_account(
        body.email.to_owned(),
        body.account_name.to_owned(),
        body.first_name.to_owned(),
        body.last_name.to_owned(),
        body.password.to_owned(),
        body.provider_name.to_owned(),
        Box::new(&*account_type_registration_repo),
        Box::new(&*account_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            GetOrCreateResponseKind::Created(record) => {
                HttpResponse::Created().json(record)
            }
            GetOrCreateResponseKind::NotCreated(record, _) => {
                HttpResponse::Ok().json(record)
            }
        },
    }
}

#[utoipa::path(
    patch,
    context_path = "/myc/default-users/accounts",
    request_body = UpdateOwnAccountNameAccountBody,
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 403,
            description = "Forbidden.",
            body = JsonError,
        ),
        (
            status = 401,
            description = "Unauthorized.",
            body = JsonError,
        ),
        (
            status = 400,
            description = "Account name not updated.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Account name successfully updated.",
            body = Account,
        ),
    ),
)]
#[patch("/{id}/update-account-name")]
pub async fn update_own_account_name_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateOwnAccountNameAccountBody>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    let profile = profile.to_profile();

    if path.to_owned() != profile.current_account_id {
        warn!("No account owner trying to perform account updating.");
        warn!(
            "Account {} trying to update {}",
            profile.current_account_id,
            path.to_owned()
        );

        return HttpResponse::Forbidden().json(JsonError::new(String::from(
            "Invalid operation. Operation restricted to account owners.",
        )));
    }

    match update_own_account_name(
        profile,
        body.name.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*account_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
        },
    }
}
