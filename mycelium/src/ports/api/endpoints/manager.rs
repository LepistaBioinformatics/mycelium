use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc::domain::dtos::{
    email::EmailDTO,
    guest::{GuestRoleDTO, GuestUserDTO, PermissionsType},
    profile::{LicensedResourcesDTO, ProfileDTO},
};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the Customer Partner API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        manager_endpoints::guest_user_url
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            EmailDTO,
            GuestUserDTO,
            GuestRoleDTO,
            LicensedResourcesDTO,
            PermissionsType,
            ProfileDTO,
        ),
    ),
    tags(
        (
            name = "manager",
            description = "Manager management endpoints."
        )
    ),
)]
pub struct ApiDoc;

// ? ---------------------------------------------------------------------------
// ? This module contained the results-expert endpoints
// ? ---------------------------------------------------------------------------

pub mod manager_endpoints {

    use crate::modules::manager::{
        AccountFetchingModule, GuestUserRegistrationModule,
        MessageSendingModule,
    };

    use actix_web::{get, web, HttpRequest, HttpResponse, Responder};
    use clean_base::entities::default_response::GetOrCreateResponseKind;
    use myc::{
        domain::{
            dtos::email::EmailDTO,
            entities::{
                manager::guest_user_registration::GuestUserRegistration,
                shared::{
                    account_fetching::AccountFetching,
                    message_sending::MessageSending,
                },
            },
        },
        public::extractor::profile_extractor,
        use_cases::managers::guest::guest_user::guest_user,
    };
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/managers")
                .service(web::scope("/guest").service(guest_user_url)),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct GuestUserParams {
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
        path = "/managers/guest/account/{account}/role/{role}",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            ("role" = Uuid, Path, description = "The guest-role unique token."),
            GuestUserParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 201,
                description = "Guest already exists done.",
                body = GuestUserDTO,
            ),
            (
                status = 200,
                description = "Guesting done.",
                body = GuestUserDTO,
            ),
        ),
    )]
    #[get("/account/{account}/role/{role}")]
    pub async fn guest_user_url(
        path: web::Path<(Uuid, Uuid)>,
        info: web::Query<GuestUserParams>,
        req: HttpRequest,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        guest_registration_repo: Inject<
            GuestUserRegistrationModule,
            dyn GuestUserRegistration,
        >,
        message_sending_repo: Inject<MessageSendingModule, dyn MessageSending>,
    ) -> impl Responder {
        let profile = match profile_extractor(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        let (account_id, role_id) = path.to_owned();

        let email = match EmailDTO::from_string(info.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest().body(err.to_string())
            }
            Ok(res) => res,
        };

        match guest_user(
            profile,
            email,
            account_id,
            role_id,
            Box::new(&*account_fetching_repo),
            Box::new(&*guest_registration_repo),
            Box::new(&*message_sending_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                GetOrCreateResponseKind::NotCreated(guest, _) => {
                    HttpResponse::Ok().json(guest)
                }
                GetOrCreateResponseKind::Created(guest) => {
                    HttpResponse::Created().json(guest)
                }
            },
        }
    }
}
