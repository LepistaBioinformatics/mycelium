use crate::{
    endpoints::{
        shared::{PaginationParams, UrlGroup},
        standard::shared::build_actor_context,
    },
    modules::{
        ErrorCodeDeletionModule, ErrorCodeFetchingModule,
        ErrorCodeRegistrationModule, ErrorCodeUpdatingModule,
    },
};

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use clean_base::entities::{FetchManyResponseKind, FetchResponseKind};
use myc_core::{
    domain::{
        actors::DefaultActor,
        dtos::native_error_codes::NativeErrorCodes,
        entities::{
            ErrorCodeDeletion, ErrorCodeFetching, ErrorCodeRegistration,
            ErrorCodeUpdating,
        },
    },
    use_cases::roles::standard::system_manager::error_codes::{
        delete_error_code, get_error_code, list_error_codes,
        register_error_code, update_error_code_message_and_details,
    },
};
use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};

// ? -----------------------------------------------------------------------
// ? Configure application
// ? -----------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(register_error_code_url)
        .service(list_error_codes_url)
        .service(get_error_code_url)
        .service(update_error_code_message_and_details_url)
        .service(delete_error_code_url);
}

// ? -----------------------------------------------------------------------
// ? Define API structs
// ? -----------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateErrorCodeBody {
    prefix: String,
    message: String,
    details: Option<String>,
    is_internal: bool,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListErrorCodesParams {
    prefix: Option<String>,
    code: Option<i32>,
    is_internal: Option<bool>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateErrorCodeMessageAndDetailsBody {
    message: String,
    details: Option<String>,
}

// ? -----------------------------------------------------------------------
// ? Define API paths
// ? -----------------------------------------------------------------------

/// Register a new error code.
///
/// This action is restricted to manager users.
#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::SystemManager, UrlGroup::ErrorCodes),
    request_body = CreateErrorCodeBody,
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
            description = "Error code already exists.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Error code created.",
            body = ErrorCode,
        ),
    ),
)]
#[post("/")]
pub async fn register_error_code_url(
    body: web::Json<CreateErrorCodeBody>,
    profile: MyceliumProfileData,
    error_code_registration_repo: Inject<
        ErrorCodeRegistrationModule,
        dyn ErrorCodeRegistration,
    >,
) -> impl Responder {
    match register_error_code(
        profile.to_profile(),
        body.prefix.to_owned(),
        body.message.to_owned(),
        body.details.to_owned(),
        body.is_internal.to_owned(),
        Box::new(&*error_code_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError().json(
            JsonError::new(err.to_string()).with_code(err.code().to_string()),
        ),
        Ok(account) => HttpResponse::Created().json(account),
    }
}

/// List available error codes.
///
/// List accounts with pagination. The `records` field contains a vector of
/// [`ErrorCode`] model.
///
#[utoipa::path(
    get,
    context_path = build_actor_context(DefaultActor::SystemManager, UrlGroup::ErrorCodes),
    params(
        ListErrorCodesParams,
        PaginationParams,
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
            status = 204,
            description = "Not found.",
        ),
        (
            status = 200,
            description = "Fetching success.",
            body = [ErrorCode],
        ),
    ),
)]
#[get("/")]
pub async fn list_error_codes_url(
    info: web::Query<ListErrorCodesParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    error_code_fetching_repo: Inject<
        ErrorCodeFetchingModule,
        dyn ErrorCodeFetching,
    >,
) -> impl Responder {
    match list_error_codes(
        profile.to_profile(),
        info.prefix.to_owned(),
        info.code.to_owned(),
        info.is_internal.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*error_code_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchManyResponseKind::NotFound => {
                HttpResponse::NoContent().finish()
            }
            FetchManyResponseKind::Found(accounts) => {
                HttpResponse::Ok().json(accounts)
            }
            FetchManyResponseKind::FoundPaginated(accounts) => {
                HttpResponse::Ok().json(accounts)
            }
        },
    }
}

#[utoipa::path(
    get,
    context_path = build_actor_context(DefaultActor::SystemManager, UrlGroup::ErrorCodes),
    params(
        ("prefix" = String, Path, description = "The error prefix."),
        ("code" = i32, Path, description = "The error code."),
    ),
    responses(
        (
            status = 500,
            description = "Unknown internal server error.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Not found.",
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
            status = 200,
            description = "Fetching success.",
            body = Account,
        ),
    ),
)]
#[get("/prefix/{account}/code/{code}")]
pub async fn get_error_code_url(
    path: web::Path<(String, i32)>,
    profile: MyceliumProfileData,
    error_code_fetching_repo: Inject<
        ErrorCodeFetchingModule,
        dyn ErrorCodeFetching,
    >,
) -> impl Responder {
    let (prefix, code) = path.into_inner();

    match get_error_code(
        profile.to_profile(),
        prefix.to_owned(),
        code.to_owned(),
        Box::new(&*error_code_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            FetchResponseKind::NotFound(_) => {
                HttpResponse::NoContent().finish()
            }
            FetchResponseKind::Found(accounts) => {
                HttpResponse::Ok().json(accounts)
            }
        },
    }
}

#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::SystemManager, UrlGroup::ErrorCodes),
    params(
        ("prefix" = String, Path, description = "The error prefix."),
        ("code" = i32, Path, description = "The error code."),
    ),
    request_body = UpdateErrorCodeMessageAndDetailsBody,
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
            description = "Error code not updated.",
            body = JsonError,
        ),
        (
            status = 202,
            description = "Error code updated.",
            body = Account,
        ),
    ),
)]
#[patch("/prefix/{account}/code/{code}")]
pub async fn update_error_code_message_and_details_url(
    path: web::Path<(String, i32)>,
    body: web::Json<UpdateErrorCodeMessageAndDetailsBody>,
    profile: MyceliumProfileData,
    error_code_fetching_repo: Inject<
        ErrorCodeFetchingModule,
        dyn ErrorCodeFetching,
    >,
    error_code_updating_repo: Inject<
        ErrorCodeUpdatingModule,
        dyn ErrorCodeUpdating,
    >,
) -> impl Responder {
    let (prefix, code) = path.into_inner();

    match update_error_code_message_and_details(
        profile.to_profile(),
        prefix,
        code,
        body.message.to_owned(),
        body.details.to_owned(),
        Box::new(&*error_code_fetching_repo),
        Box::new(&*error_code_updating_repo),
    )
    .await
    {
        Err(err) => {
            let target_msg = NativeErrorCodes::MYC00005.as_str();
            if err.is_in(vec![target_msg]) {
                return HttpResponse::BadRequest().json(
                    JsonError::new(err.to_string()).with_code_str(target_msg),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()))
        }
        Ok(res) => HttpResponse::Accepted().json(res),
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(DefaultActor::SystemManager, UrlGroup::ErrorCodes),
    params(
        ("prefix" = String, Path, description = "The error prefix."),
        ("code" = i32, Path, description = "The error code."),
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
            description = "Error code not deleted.",
            body = JsonError,
        ),
        (
            status = 204,
            description = "Error code deleted.",
        ),
    ),
)]
#[delete("/prefix/{account}/code/{code}")]
pub async fn delete_error_code_url(
    path: web::Path<(String, i32)>,
    profile: MyceliumProfileData,
    error_code_deletion_repo: Inject<
        ErrorCodeDeletionModule,
        dyn ErrorCodeDeletion,
    >,
) -> impl Responder {
    let (prefix, code) = path.into_inner();

    match delete_error_code(
        profile.to_profile(),
        prefix,
        code,
        Box::new(&*error_code_deletion_repo),
    )
    .await
    {
        Err(err) => {
            let target_msg = NativeErrorCodes::MYC00007.as_str();

            if err.is_in(vec![target_msg]) {
                return HttpResponse::Forbidden().json(
                    JsonError::new(err.to_string()).with_code_str(target_msg),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()))
        }
        Ok(_) => HttpResponse::NoContent().finish(),
    }
}
