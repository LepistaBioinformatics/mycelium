use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::domain::dtos::{
    account::{Account, AccountType},
    email::Email,
    guest::{GuestRole, GuestUser, PermissionsType},
    profile::{LicensedResources, Profile},
};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the Customer Partner API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        manager_endpoints::create_subscription_account_url,
        manager_endpoints::guest_user_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account,
            AccountType,
            Email,
            GuestUser,
            GuestRole,
            LicensedResources,
            PermissionsType,
            Profile,
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

    use crate::modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountTypeRegistrationModule, GuestUserRegistrationModule,
        MessageSendingModule, UserRegistrationModule,
    };

    use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
    use clean_base::entities::default_response::GetOrCreateResponseKind;
    use myc_core::{
        domain::{
            dtos::email::Email,
            entities::{
                AccountFetching, AccountRegistration, AccountTypeRegistration,
                GuestUserRegistration, MessageSending, UserRegistration,
            },
        },
        use_cases::managers::{
            account::create_subscription_account, guest::guest_user,
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
            web::scope("/managers")
                .service(
                    web::scope("/account")
                        .service(create_subscription_account_url),
                )
                .service(web::scope("/guest").service(guest_user_url)),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateSubscriptionAccountParams {
        pub email: String,
        pub account_name: String,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct GuestUserParams {
        pub email: String,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Account
    //
    // ? -----------------------------------------------------------------------

    /// Create Subscription Account
    ///
    /// Subscription accounts represents shared entities, like institutions,
    /// groups, but not real persons.
    #[utoipa::path(
        post,
        path = "/managers/account/",
        params(
            CreateSubscriptionAccountParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 201,
                description = "Account created.",
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
    pub async fn create_subscription_account_url(
        info: web::Query<CreateSubscriptionAccountParams>,
        req: HttpRequest,
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
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match create_subscription_account(
            profile,
            info.email.to_owned(),
            info.account_name.to_owned(),
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
                GetOrCreateResponseKind::NotCreated(guest, _) => {
                    HttpResponse::Ok().json(guest)
                }
                GetOrCreateResponseKind::Created(guest) => {
                    HttpResponse::Created().json(guest)
                }
            },
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Guest
    //
    // ? -----------------------------------------------------------------------

    /// Guest a user to work on account.
    ///
    /// This action gives the ability of the target account (specified through
    /// the `account` argument) to perform actions specified in the `role`
    /// path argument.
    #[utoipa::path(
        post,
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
                description = "Guesting done.",
                body = GuestUser,
            ),
            (
                status = 200,
                description = "Guest already exist.",
                body = GuestUser,
            ),
        ),
    )]
    #[post("/account/{account}/role/{role}")]
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
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        let (account_id, role_id) = path.to_owned();

        let email = match Email::from_string(info.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest().body(err.to_string())
            }
            Ok(res) => res,
        };

        match guest_user(
            profile,
            email,
            role_id,
            account_id,
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

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Guest Role
    //
    // ? -----------------------------------------------------------------------

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Guest Role
    //
    // ? -----------------------------------------------------------------------
}
