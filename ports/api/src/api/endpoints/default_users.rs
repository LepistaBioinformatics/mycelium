use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::domain::dtos::{
    account::{Account, AccountType},
    profile::{LicensedResources, Profile},
};
use myc_http_tools::utils::JsonError;
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        account_endpoints::create_default_account_url,
        account_endpoints::update_own_account_name_url,
        profile_endpoints::fetch_profile_from_email_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account,
            AccountType,
            JsonError,
            LicensedResources,
            Profile,
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

pub mod account_endpoints {

    use crate::modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountTypeRegistrationModule, AccountUpdatingModule,
        UserRegistrationModule,
    };

    use actix_web::{patch, post, web, HttpRequest, HttpResponse, Responder};
    use clean_base::entities::default_response::{
        GetOrCreateResponseKind, UpdatingResponseKind,
    };
    use log::warn;
    use myc_core::{
        domain::entities::{
            AccountFetching, AccountRegistration, AccountTypeRegistration,
            AccountUpdating, UserRegistration,
        },
        use_cases::default_users::account::{
            create_default_account, update_own_account_name,
        },
    };
    use myc_http_tools::{extractor::extract_profile, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
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
        path = "/default-users/accounts/{id}/update-account-name",
        params(
            ("id" = Uuid, Path, description = "The account primary key."),
            UpdateOwnAccountNameAccountParams,
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
                .json(JsonError::new(String::from(
                "Invalid operation. Operation restricted to account owners.",
            )));
        }

        match update_own_account_name(
            profile,
            info.name.to_owned(),
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
}

pub mod profile_endpoints {

    use crate::modules::{
        LicensedResourcesFetchingModule, ProfileFetchingModule,
    };

    use actix_web::{get, web, HttpResponse, Responder};
    use myc_core::{
        domain::entities::{LicensedResourcesFetching, ProfileFetching},
        use_cases::service::profile::{
            fetch_profile_from_email, ProfileResponse,
        },
    };
    use myc_http_tools::{utils::JsonError, Email};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/profiles").service(fetch_profile_from_email_url),
        );
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
    // Account
    //
    // ? -----------------------------------------------------------------------

    #[utoipa::path(
        get,
        path = "/default-users/profiles/",
        params(
            GetProfileParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = JsonError,
            ),
            (
                status = 404,
                description = "Not found.",
                body = JsonError,
            ),
            (
                status = 400,
                description = "Bad request.",
                body = JsonError,
            ),
            (
                status = 200,
                description = "Profile fetching done.",
                body = Profile,
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
        licensed_resources_fetching_repo: Inject<
            LicensedResourcesFetchingModule,
            dyn LicensedResourcesFetching,
        >,
    ) -> impl Responder {
        let email = match Email::from_string(info.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest()
                    .json(JsonError::new(err.to_string()))
            }
            Ok(res) => res,
        };

        match fetch_profile_from_email(
            email,
            Box::new(&*profile_fetching_repo),
            Box::new(&*licensed_resources_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                ProfileResponse::UnregisteredUser(email) => {
                    HttpResponse::NotFound()
                        .json(JsonError::new(email.get_email()))
                }
                ProfileResponse::RegisteredUser(profile) => {
                    HttpResponse::Ok().json(profile)
                }
            },
        }
    }
}
