use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::{
    domain::dtos::{
        email::Email,
        guest::PermissionsType,
        profile::{LicensedResources, Profile},
    },
    use_cases::service::profile::{ProfilePack, ProfileResponse},
};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        service_endpoints::fetch_profile_from_email_url,
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
            ProfileResponse,
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

pub mod service_endpoints {
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
            web::scope("/profile").service(fetch_profile_from_email_url),
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
                body = ProfileResponse,
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
