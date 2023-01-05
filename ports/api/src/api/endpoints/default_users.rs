use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use utoipa::OpenApi;
use myc_core::domain::dtos::account::{Account, AccountType};

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        default_user_endpoints::create_default_account_url,
        default_user_endpoints::update_own_account_name_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account, 
            AccountType,
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

    use crate::modules::{
        AccountRegistrationModule, AccountTypeRegistrationModule,
        UserRegistrationModule, AccountFetchingModule, AccountUpdatingModule,
    };

    use actix_web::{patch, post, web, HttpResponse, Responder, HttpRequest};
    use clean_base::entities::default_response::{
        GetOrCreateResponseKind, UpdatingResponseKind
    };
    use log::warn;
    use myc_core::{
        domain::entities::{
            UserRegistration,
            AccountTypeRegistration,
            AccountRegistration, AccountFetching, AccountUpdating,
        },
        use_cases::default_users::account::{
            create_default_account, update_own_account_name
        },
    };
    use myc_http_tools::extractor::extract_profile;
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/default-users")
                .service(web::scope("/accounts")
                    .service(create_default_account_url))
                    .service(update_own_account_name_url),
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

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateOwnAccountNameAccountParams {
        pub name: String,
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

    #[utoipa::path(
        patch,
        path = "/default-users/accounts/{id}/update-account-name",
        params(
            UpdateOwnAccountNameAccountParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Account name not updated.",
                body = String,
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
        info: web::Query<UpdateOwnAccountNameAccountParams>,
        req: HttpRequest,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        if path.to_owned() != profile.current_account_id {
            warn!("No account owner trying to perform account updating.");
            warn!(
                "Account {} trying to update {}", 
                profile.current_account_id, 
                path.to_owned()
            );

            return HttpResponse::Forbidden()
                .body(String::from(
                    "Invalid operation. Operation restricted to account owners."
                ));
        }

        match update_own_account_name(
            profile,
            info.name.to_owned(),
            Box::new(&*account_fetching_repo),
            Box::new(&*account_updating_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                UpdatingResponseKind::NotUpdated(_, msg) => {
                    HttpResponse::BadRequest().body(msg)
                }
                UpdatingResponseKind::Updated(record) => {
                    HttpResponse::Accepted().json(record)
                }
            },
        }
    }
}
