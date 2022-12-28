use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use utoipa::OpenApi;
use myc_core::domain::dtos::account::{AccountDTO, AccountTypeDTO};

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        default_user_endpoints::create_default_account_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            AccountDTO, 
            AccountTypeDTO,
        ),
    ),
    tags(
        (
            name = "default-users",
            description = "Default Users management endpoints."
        )
    ),
)]
pub struct ApiDoc;

// ? ---------------------------------------------------------------------------
// ? Create endpoints module
// ? ---------------------------------------------------------------------------

pub mod default_user_endpoints {

    use actix_web::{post, web, HttpResponse, Responder};
    use clean_base::entities::default_response::GetOrCreateResponseKind;
    use myc_core::{
        domain::entities::{
            UserRegistration,
            AccountTypeRegistration,
            AccountRegistration,
        },
        use_cases::default_users::account::create_default_account::create_default_account,
    };
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;

    use crate::modules::{
        AccountRegistrationModule, AccountTypeRegistrationModule,
        UserRegistrationModule,
    };

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/default-users")
                .service(web::scope("/accounts")
                    .service(create_default_account_url)),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateDefaultAccountParams {
        pub email: String,
        pub account_name: String,
        pub first_name: Option<String>,
        pub last_name: Option<String>,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Account
    //
    // ? -----------------------------------------------------------------------

    #[utoipa::path(
        post,
        path = "/default-users/accounts/",
        params(
            CreateDefaultAccountParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 201,
                description = "Account successfully created.",
                body = AccountDTO,
            ),
            (
                status = 200,
                description = "Account already exists.",
                body = AccountDTO,
            ),
        ),
    )]
    #[post("/")]
    pub async fn create_default_account_url(
        info: web::Query<CreateDefaultAccountParams>,
        user_registration_repo: Inject<
            UserRegistrationModule,
            dyn UserRegistration,
        >,
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
            info.email.to_owned(),
            info.account_name.to_owned(),
            info.first_name.to_owned(),
            info.last_name.to_owned(),
            Box::new(&*user_registration_repo),
            Box::new(&*account_type_registration_repo),
            Box::new(&*account_registration_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
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
}
