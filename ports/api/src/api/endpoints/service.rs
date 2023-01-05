use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::{
    domain::dtos::{
        email::Email,
        guest::PermissionsType,
        profile::{LicensedResources, Profile},
        token::Token,
    },
    use_cases::service::profile::ProfilePack,
};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        profile_endpoints::fetch_profile_from_email_url,
        token_endpoints::clean_tokens_range_url,
        token_endpoints::validate_token_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Email,
            LicensedResources,
            PermissionsType,
            Profile,
            ProfilePack,
            Token,
        ),
    ),
    tags(
        (
            name = "service",
            description = "Service management endpoints."
        )
    ),
)]
pub struct ApiDoc;

// ? ---------------------------------------------------------------------------
// ? Create endpoints module
// ? ---------------------------------------------------------------------------

pub mod profile_endpoints {
    use crate::modules::{ProfileFetchingModule, TokenRegistrationModule};

    use actix_web::{get, web, HttpResponse, Responder};
    use myc_core::{
        domain::{
            dtos::email::Email,
            entities::{ProfileFetching, TokenRegistration},
        },
        use_cases::service::profile::{
            fetch_profile_from_email, ProfileResponse,
        },
    };
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(web::scope("/service").service(
            web::scope("/profiles").service(fetch_profile_from_email_url),
        ));
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct GetProfileParams {
        pub email: String,
        pub service: String,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Profile
    //
    // ? -----------------------------------------------------------------------

    #[utoipa::path(
        get,
        path = "/service/profile/",
        params(
            GetProfileParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 404,
                description = "Not found.",
                body = String,
            ),
            (
                status = 400,
                description = "Bad request.",
                body = String,
            ),
            (
                status = 200,
                description = "Profile fetching done.",
                body = ProfilePack,
            ),
        ),
    )]
    #[get("/")]
    pub async fn fetch_profile_from_email_url(
        info: web::Query<GetProfileParams>,
        profile_fetching_repo: Inject<
            ProfileFetchingModule,
            dyn ProfileFetching,
        >,
        token_registration_repo: Inject<
            TokenRegistrationModule,
            dyn TokenRegistration,
        >,
    ) -> impl Responder {
        let email = match Email::from_string(info.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest().body(err.to_string())
            }
            Ok(res) => res,
        };

        match fetch_profile_from_email(
            email,
            info.service.to_owned(),
            Box::new(&*profile_fetching_repo),
            Box::new(&*token_registration_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                ProfileResponse::UnregisteredUser(email) => {
                    HttpResponse::NotFound().body(email.get_email())
                }
                ProfileResponse::RegisteredUser(profile) => {
                    HttpResponse::Ok().json(profile)
                }
            },
        }
    }
}

pub mod token_endpoints {
    use crate::modules::{TokenCleanupModule, TokenDeregistrationModule};

    use actix_web::{get, post, web, HttpResponse, Responder};
    use clean_base::entities::default_response::{
        DeletionManyResponseKind, FetchResponseKind,
    };
    use myc_core::{
        domain::entities::{TokenCleanup, TokenDeregistration},
        use_cases::service::token::{clean_tokens_range, validate_token},
    };
    use serde::{Deserialize, Serialize};
    use shaku_actix::Inject;
    use utoipa::IntoParams;
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/service").service(
                web::scope("/token")
                    .service(clean_tokens_range_url)
                    .service(validate_token_url),
            ),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct ValidateTokenParams {
        pub service: String,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Token
    //
    // ? -----------------------------------------------------------------------

    /// Cleanup token list
    ///
    /// Perform a cleanup on the token list. This endpoint should be exposed to
    /// the system only.
    #[utoipa::path(
        post,
        path = "/service/token/cleanup-tokens/",
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Bad request.",
                body = String,
            ),
            (
                status = 200,
                description = "Cleanup done.",
                body = i64,
            ),
        ),
    )]
    #[post("/cleanup-tokens/")]
    pub async fn clean_tokens_range_url(
        token_cleanup_repo: Inject<TokenCleanupModule, dyn TokenCleanup>,
    ) -> impl Responder {
        #[derive(Serialize)]
        struct Info {
            msg: String,
        }

        match clean_tokens_range(Box::new(&*token_cleanup_repo)).await {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                DeletionManyResponseKind::NotDeleted(_, msg) => {
                    HttpResponse::BadRequest().body(msg)
                }
                DeletionManyResponseKind::Deleted(records) => {
                    HttpResponse::Ok().body(records.to_string())
                }
            },
        }
    }

    /// Fetch validation token
    ///
    /// Try to fetch a token. If exists return a token object.
    #[utoipa::path(
        get,
        path = "/service/token/{token}",
        params(
            ("token" = Uuid, Path, description = "The token itself."),
            ValidateTokenParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 404,
                description = "Not found.",
                body = String,
            ),
            (
                status = 200,
                description = "Token fetching done.",
                body = Token,
            ),
        ),
    )]
    #[get("/{token}")]
    pub async fn validate_token_url(
        path: web::Path<Uuid>,
        info: web::Query<ValidateTokenParams>,
        token_deregistration_repo: Inject<
            TokenDeregistrationModule,
            dyn TokenDeregistration,
        >,
    ) -> impl Responder {
        match validate_token(
            path.to_owned(),
            info.service.to_owned(),
            Box::new(&*token_deregistration_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                FetchResponseKind::NotFound(token) => {
                    HttpResponse::NotFound().body(token.unwrap().to_string())
                }
                FetchResponseKind::Found(token) => {
                    HttpResponse::Ok().json(token)
                }
            },
        }
    }
}
