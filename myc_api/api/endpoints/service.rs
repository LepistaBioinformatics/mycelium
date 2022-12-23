use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::domain::dtos::{
    email::EmailDTO,
    guest::PermissionsType,
    profile::{LicensedResourcesDTO, ProfileDTO},
};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the Customer Partner API documentation
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
            EmailDTO,
            LicensedResourcesDTO,
            PermissionsType,
            ProfileDTO,
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
// ? This module contained the results-expert endpoints
// ? ---------------------------------------------------------------------------

pub mod service_endpoints {
    use crate::modules::service::ProfileFetchingModule;

    use actix_web::{get, web, HttpResponse, Responder};
    use clean_base::entities::default_response::FetchResponseKind;
    use myc_core::{
        domain::{
            dtos::email::EmailDTO,
            entities::service::profile_fetching::ProfileFetching,
        },
        use_cases::service::fetch_profile_from_email::fetch_profile_from_email,
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
                body = ProfileDTO,
            ),
        ),
    )]
    #[get("/")]
    pub async fn fetch_profile_from_email_url(
        info: web::Query<GetProfileParams>,
        repo: Inject<ProfileFetchingModule, dyn ProfileFetching>,
    ) -> impl Responder {
        let email = match EmailDTO::from_string(info.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest().body(err.to_string())
            }
            Ok(res) => res,
        };

        match fetch_profile_from_email(email, Box::new(&*repo)).await {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                FetchResponseKind::NotFound(email) => {
                    HttpResponse::NotFound().body(email.unwrap().get_email())
                }
                FetchResponseKind::Found(records) => {
                    HttpResponse::Ok().json(records)
                }
            },
        }
    }
}
