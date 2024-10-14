use crate::{
    dtos::MyceliumProfileData,
    endpoints::shared::{PaginationParams, UrlGroup, UrlScope},
    modules::{
        TenantDeletionModule, TenantFetchingModule, TenantRegistrationModule,
        TenantUpdatingModule, UserFetchingModule,
    },
};

use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        dtos::{native_error_codes::NativeErrorCodes, tenant::TenantMetaKey},
        entities::{
            TenantDeletion, TenantFetching, TenantRegistration, TenantUpdating,
            UserFetching,
        },
    },
    use_cases::roles::managers::{
        create_tenant, delete_tenant, exclude_tenant_owner,
        include_tenant_owner, list_tenant,
    },
};
use myc_http_tools::utils::JsonError;
use mycelium_base::entities::{
    CreateResponseKind, DeletionResponseKind, FetchManyResponseKind,
};
use serde::Deserialize;
use shaku_actix::Inject;
use utoipa::{IntoParams, ToSchema};
use uuid::Uuid;

// ? ---------------------------------------------------------------------------
// ? Configure application
// ? ---------------------------------------------------------------------------

pub fn configure(config: &mut web::ServiceConfig) {
    config.service(
        web::scope("/tenants")
            .service(create_tenant_url)
            .service(list_tenant_url)
            .service(delete_tenant_url)
            .service(include_tenant_owner_url)
            .service(exclude_tenant_owner_url),
    );
}

// ? ---------------------------------------------------------------------------
// ? Define API structs
// ? ---------------------------------------------------------------------------

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateTenantBody {
    /// The name of the tenant
    name: String,

    /// The description of the tenant
    description: Option<String>,

    /// The owner of the tenant
    owner_id: Uuid,
}

#[derive(Deserialize, IntoParams)]
#[serde(rename_all = "camelCase")]
pub struct ListTenantParams {
    name: Option<String>,
    owner: Option<Uuid>,
    metadata_key: Option<TenantMetaKey>,
    status_verified: Option<bool>,
    status_archived: Option<bool>,
    status_trashed: Option<bool>,
    tag_value: Option<String>,
    tag_meta: Option<String>,
}

// ? ---------------------------------------------------------------------------
// ? Define API paths
// ? ---------------------------------------------------------------------------

#[utoipa::path(
    post,
    context_path = UrlGroup::Tenants.with_scope(UrlScope::Managers),
    request_body = CreateTenantBody,
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
            description = "Tenant already exists.",
            body = JsonError,
        ),
        (
            status = 201,
            description = "Tenant created.",
            body = Tenant,
        ),
    ),
)]
#[post("/")]
pub async fn create_tenant_url(
    body: web::Json<CreateTenantBody>,
    profile: MyceliumProfileData,
    user_fetching_repo: Inject<UserFetchingModule, dyn UserFetching>,
    tenant_registration_repo: Inject<
        TenantRegistrationModule,
        dyn TenantRegistration,
    >,
) -> impl Responder {
    match create_tenant(
        profile.to_profile(),
        body.name.clone(),
        body.description.clone(),
        body.owner_id,
        Box::new(&*user_fetching_repo),
        Box::new(&*tenant_registration_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![NativeErrorCodes::MYC00014]) {
                return HttpResponse::Conflict().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(res) => match res {
            CreateResponseKind::NotCreated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            CreateResponseKind::Created(tenant) => {
                HttpResponse::Created().json(tenant)
            }
        },
    }
}

#[utoipa::path(
    get,
    context_path = UrlGroup::Tenants.with_scope(UrlScope::Managers),
    params(
        ListTenantParams,
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
pub async fn list_tenant_url(
    info: web::Query<ListTenantParams>,
    page: web::Query<PaginationParams>,
    profile: MyceliumProfileData,
    tenant_fetching_repo: Inject<TenantFetchingModule, dyn TenantFetching>,
) -> impl Responder {
    match list_tenant(
        profile.to_profile(),
        info.name.to_owned(),
        info.owner.to_owned(),
        info.metadata_key.to_owned(),
        info.status_verified.to_owned(),
        info.status_archived.to_owned(),
        info.status_trashed.to_owned(),
        info.tag_value.to_owned(),
        info.tag_meta.to_owned(),
        page.page_size.to_owned(),
        page.skip.to_owned(),
        Box::new(&*tenant_fetching_repo),
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
    delete,
    context_path = UrlGroup::Tenants.with_scope(UrlScope::Managers),
    params(
        ("id" = Uuid, Path, description = "The tenant primary key."),
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
            description = "Tenant successfully registered.",
            body = AnalysisTag,
        ),
    ),
)]
#[delete("/{id}")]
pub async fn delete_tenant_url(
    profile: MyceliumProfileData,
    path: web::Path<Uuid>,
    tenant_deletion_repo: Inject<TenantDeletionModule, dyn TenantDeletion>,
) -> impl Responder {
    match delete_tenant(
        profile.to_profile(),
        path.into_inner(),
        Box::from(&*tenant_deletion_repo),
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

#[utoipa::path(
    patch,
    context_path = UrlGroup::Tenants.with_scope(UrlScope::Managers),
    params(
        ("id" = Uuid, Path, description = "The tenant primary key."),
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
            status = 201,
            description = "Owner included.",
            body = TenantOwnerConnection,
        ),
    ),
)]
#[patch("/{id}/owner/{owner_id}")]
pub async fn include_tenant_owner_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    tenant_updating_repo: Inject<TenantUpdatingModule, dyn TenantUpdating>,
) -> impl Responder {
    let (tenant_id, owner_id) = path.into_inner();

    match include_tenant_owner(
        profile.to_profile(),
        tenant_id,
        owner_id,
        Box::new(&*tenant_updating_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![NativeErrorCodes::MYC00015]) {
                return HttpResponse::Conflict().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(res) => match res {
            CreateResponseKind::NotCreated(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            CreateResponseKind::Created(tenant) => {
                HttpResponse::Created().json(tenant)
            }
        },
    }
}

#[utoipa::path(
    patch,
    context_path = UrlGroup::Tenants.with_scope(UrlScope::Managers),
    params(
        ("id" = Uuid, Path, description = "The tenant primary key."),
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
            description = "Owner deleted.",
        ),
    ),
)]
#[delete("/{id}/owner/{owner_id}")]
pub async fn exclude_tenant_owner_url(
    path: web::Path<(Uuid, Uuid)>,
    profile: MyceliumProfileData,
    tenant_deletion_repo: Inject<TenantDeletionModule, dyn TenantDeletion>,
) -> impl Responder {
    let (tenant_id, owner_id) = path.into_inner();

    match exclude_tenant_owner(
        profile.to_profile(),
        tenant_id,
        owner_id,
        Box::new(&*tenant_deletion_repo),
    )
    .await
    {
        Err(err) => {
            let code_string = err.code().to_string();

            if err.is_in(vec![NativeErrorCodes::MYC00016]) {
                return HttpResponse::Conflict().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                );
            }

            HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string()).with_code(code_string))
        }
        Ok(res) => match res {
            DeletionResponseKind::NotDeleted(_, msg) => {
                HttpResponse::BadRequest().json(JsonError::new(msg))
            }
            DeletionResponseKind::Deleted => HttpResponse::NoContent().finish(),
        },
    }
}
