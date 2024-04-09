use crate::{
    dtos::MyceliumProfileData,
    endpoints::{
        shared::{PaginationParams, UrlGroup},
        standard::shared::build_actor_context,
    },
    modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountTypeRegistrationModule, AccountUpdatingModule,
        TagDeletionModule, TagRegistrationModule, TagUpdatingModule,
        WebHookFetchingModule,
    },
};

use actix_web::{delete, get, patch, post, put, web, HttpResponse, Responder};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use myc_core::{
    domain::{
        actors::DefaultActor,
        dtos::{
            account::VerboseStatus, native_error_codes::NativeErrorCodes,
            tag::Tag,
        },
        entities::{
            AccountFetching, AccountRegistration, AccountTypeRegistration,
            AccountUpdating, TagDeletion, TagRegistration, TagUpdating,
            WebHookFetching,
        },
    },
    use_cases::roles::standard::subscription_account_manager::{
        account::{
            create_subscription_account, get_account_details,
            list_accounts_by_type, propagate_existing_subscription_account,
            update_account_name_and_flags,
        },
        tag::{delete_tag, register_tag, update_tag},
    },
};
use myc_http_tools::utils::JsonError;
use mycelium_base::entities::{
    DeletionResponseKind, FetchManyResponseKind, FetchResponseKind,
    GetOrCreateResponseKind, UpdatingResponseKind,
};
use serde::Deserialize;
use shaku_actix::Inject;
use std::collections::HashMap;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config
        .service(create_subscription_account_url)
        .service(update_account_name_and_flags_url)
        .service(list_accounts_by_type_url)
        .service(get_account_details_url)
        .service(propagate_existing_subscription_account_url)
        .service(register_tag_url)
        .service(update_tag_url)
        .service(delete_tag_url);
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateSubscriptionAccountBody {
    account_name: String,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSubscriptionAccountNameAndFlagsBody {
    name: Option<String>,
    is_active: Option<bool>,
    is_checked: Option<bool>,
    is_archived: Option<bool>,
    is_default: Option<bool>,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListSubscriptionAccountParams {
    term: Option<String>,
    tag_value: Option<String>,
    is_subscription: Option<bool>,
    is_owner_active: Option<bool>,
    status: Option<VerboseStatus>,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTagBody {
    value: String,
    meta: HashMap<String, String>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
//
// Account
//
// ? ---------------------------------------------------------------------------

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    request_body = CreateSubscriptionAccountBody,
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
            description = "Account already exists.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Account created.",
            body = CreateSubscriptionResponse,
        ),
    ),
)]
#[post("/")]
pub async fn create_subscription_account_url(
    auth: BearerAuth,
    body: web::Json<CreateSubscriptionAccountBody>,
    profile: MyceliumProfileData,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
    account_registration_repo: Inject<
        AccountRegistrationModule,
        dyn AccountRegistration,
    >,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    match create_subscription_account(
        profile.to_profile(),
        auth.token().to_owned(),
        body.account_name.to_owned(),
        Box::new(&*account_type_registration_repo),
        Box::new(&*account_registration_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![
                NativeErrorCodes::MYC00002.as_str(),
                NativeErrorCodes::MYC00003.as_str(),
            ]) {
                return HttpResponse::Conflict().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(account) => HttpResponse::Created().json(account),
    }
}

/// List account given an account-type
///
/// Get a filtered (or not) list of accounts.
///
/// List accounts with pagination. The `records` field contains a vector of
/// [`Account`] model.
///
#[utoipa::path(
    get,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ListSubscriptionAccountParams,
        PaginationParams,
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
            body = [Account],
        ),
    ),
)]
#[get("/")]
pub async fn list_accounts_by_type_url(
    info: web::Query<ListSubscriptionAccountParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_type_registration_repo: Inject<
        AccountTypeRegistrationModule,
        dyn AccountTypeRegistration,
    >,
) -> impl Responder {
    let mut is_account_active: Option<bool> = None;
    let mut is_account_checked: Option<bool> = None;
    let mut is_account_archived: Option<bool> = None;

    match info.status.to_owned() {
        Some(res) => {
            let flags = match res.to_flags() {
                Err(err) => {
                    return HttpResponse::InternalServerError()
                        .json(JsonError::new(err.to_string()))
                }
                Ok(res) => res,
            };

            is_account_active = flags.is_active;
            is_account_checked = flags.is_checked;
            is_account_archived = flags.is_archived;
        }
        _ => (),
    }

    match list_accounts_by_type(
        profile.to_profile(),
        info.term.to_owned(),
        info.is_owner_active.to_owned(),
        is_account_active,
        is_account_checked,
        is_account_archived,
        info.is_subscription.to_owned(),
        info.tag_value.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*account_fetching_repo),
        Box::new(&*account_type_registration_repo),
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

/// Get Subscription Account
///
/// Get a single subscription account.
#[utoipa::path(
    get,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
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
#[get("/{account}")]
pub async fn get_account_details_url(
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
) -> impl Responder {
    match get_account_details(
        profile.to_profile(),
        *path,
        Box::new(&*account_fetching_repo),
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

/// Create Subscription Account
///
/// Subscription accounts represents shared entities, like institutions,
/// groups, but not real persons.
#[utoipa::path(
    patch,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
    ),
    request_body = UpdateSubscriptionAccountNameAndFlagsBody,
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
            description = "Account already exists.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Account created.",
            body = CreateSubscriptionResponse,
        ),
    ),
)]
#[patch("/{account}")]
pub async fn update_account_name_and_flags_url(
    path: web::Path<Uuid>,
    body: web::Json<UpdateSubscriptionAccountNameAndFlagsBody>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    account_updating_repo: Inject<AccountUpdatingModule, dyn AccountUpdating>,
) -> impl Responder {
    match update_account_name_and_flags(
        profile.to_profile(),
        *path,
        body.name.to_owned(),
        body.is_active.to_owned(),
        body.is_checked.to_owned(),
        body.is_archived.to_owned(),
        body.is_default.to_owned(),
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
            UpdatingResponseKind::Updated(account) => {
                HttpResponse::Ok().json(account)
            }
        },
    }
}

/// Propagate Subscription Account
///
/// Propagate a single subscription account.
#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ("account" = Uuid, Path, description = "The account primary key."),
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
            description = "Propagating success.",
            body = Account,
        ),
    ),
)]
#[post("/{account}/propagate")]
pub async fn propagate_existing_subscription_account_url(
    auth: BearerAuth,
    path: web::Path<Uuid>,
    profile: MyceliumProfileData,
    account_fetching_repo: Inject<AccountFetchingModule, dyn AccountFetching>,
    webhook_fetching_repo: Inject<WebHookFetchingModule, dyn WebHookFetching>,
) -> impl Responder {
    match propagate_existing_subscription_account(
        profile.to_profile(),
        auth.token().to_owned(),
        *path,
        Box::new(&*account_fetching_repo),
        Box::new(&*webhook_fetching_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => HttpResponse::Ok().json(res),
    }
}

#[utoipa::path(
    post,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
    ),
    request_body = CreateTagBody,
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
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[post("/{id}/tags/")]
pub async fn register_tag_url(
    profile: MyceliumProfileData,
    path: web::Path<(Uuid,)>,
    body: web::Json<CreateTagBody>,
    tag_registration_repo: Inject<TagRegistrationModule, dyn TagRegistration>,
) -> impl Responder {
    match register_tag(
        profile.to_profile(),
        body.value.to_owned(),
        body.meta.to_owned(),
        path.into_inner().0,
        Box::from(&*tag_registration_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            GetOrCreateResponseKind::Created(record) => {
                HttpResponse::Created().json(record)
            }
            GetOrCreateResponseKind::NotCreated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
        },
    }
}

#[utoipa::path(
    put,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
        ("tag_id" = Uuid, Path, description = "The tag primary key."),
    ),
    request_body = CreateTagBody,
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
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[put("/{id}/tags/{tag_id}")]
pub async fn update_tag_url(
    profile: MyceliumProfileData,
    path: web::Path<(Uuid, Uuid)>,
    body: web::Json<CreateTagBody>,
    tag_updating_repo: Inject<TagUpdatingModule, dyn TagUpdating>,
) -> impl Responder {
    match update_tag(
        profile.to_profile(),
        Tag {
            id: path.into_inner().1,
            value: body.value.to_owned(),
            meta: Some(body.meta.to_owned()),
        },
        Box::from(&*tag_updating_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            UpdatingResponseKind::Updated(record) => {
                HttpResponse::Accepted().json(record)
            }
            UpdatingResponseKind::NotUpdated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
        },
    }
}

#[utoipa::path(
    delete,
    context_path = build_actor_context(DefaultActor::SubscriptionAccountManager, UrlGroup::Accounts),
    params(
        ("id" = Uuid, Path, description = "The account primary key."),
        ("tag_id" = Uuid, Path, description = "The tag primary key."),
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
            description = "Bad request.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tag successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[delete("/{id}/tags/{tag_id}")]
pub async fn delete_tag_url(
    profile: MyceliumProfileData,
    path: web::Path<(Uuid, Uuid)>,
    tag_deletion_repo: Inject<TagDeletionModule, dyn TagDeletion>,
) -> impl Responder {
    match delete_tag(
        profile.to_profile(),
        path.into_inner().1,
        Box::from(&*tag_deletion_repo),
    )
    .await
    {
        Err(err) => HttpResponse::InternalServerError()
            .json(JsonError::new(err.to_string())),
        Ok(res) => match res {
            DeletionResponseKind::Deleted => HttpResponse::NoContent().finish(),
            DeletionResponseKind::NotDeleted(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
        },
    }
}
