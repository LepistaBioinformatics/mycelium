use clean_base::dtos::{Children, PaginatedRecord, Parent};
use myc_core::{
    domain::dtos::{
        account::{Account, AccountType, AccountTypeEnum, VerboseStatus},
        email::Email,
        error_code::ErrorCode,
        guest::{GuestRole, GuestUser, PermissionsType},
        profile::{LicensedResources, Profile},
        role::Role,
    },
    use_cases::roles::managers::guest_role::ActionType,
};
use myc_http_tools::utils::JsonError;
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the Customer Partner API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        account_endpoints::create_subscription_account_url,
        account_endpoints::list_accounts_by_type_url,
        account_endpoints::get_account_details_url,
        account_endpoints::approve_account_url,
        account_endpoints::disapprove_account_url,
        account_endpoints::activate_account_url,
        account_endpoints::deactivate_account_url,
        account_endpoints::archive_account_url,
        account_endpoints::unarchive_account_url,
        error_code_endpoints::register_error_code_url,
        error_code_endpoints::list_error_codes_url,
        error_code_endpoints::get_error_code_url,
        error_code_endpoints::update_error_code_message_and_details_url,
        error_code_endpoints::delete_error_code_url,
        guest_endpoints::list_licensed_accounts_of_email_url,
        guest_endpoints::guest_user_url,
        guest_endpoints::uninvite_guest_url,
        guest_endpoints::update_user_guest_role_url,
        guest_endpoints::list_guest_on_subscription_account_url,
        guest_role_endpoints::crate_guest_role_url,
        guest_role_endpoints::list_guest_roles_url,
        guest_role_endpoints::delete_guest_role_url,
        guest_role_endpoints::update_guest_role_name_and_description_url,
        guest_role_endpoints::update_guest_role_permissions_url,
        role_endpoints::crate_role_url,
        role_endpoints::list_roles_url,
        role_endpoints::delete_role_url,
        role_endpoints::update_role_name_and_description_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            Children<String, String>,
            Parent<String, String>,

            // Schema models.
            Account,
            AccountType,
            AccountTypeEnum,
            ActionType,
            Email,
            ErrorCode,
            GuestUser,
            GuestRole,
            JsonError,
            LicensedResources,
            PaginatedRecord<Account>,
            PaginatedRecord<ErrorCode>,
            PermissionsType,
            Profile,
            Role,
            VerboseStatus,
            account_endpoints::CreateSubscriptionAccountBody,
            error_code_endpoints::CreateErrorCodeBody,
            error_code_endpoints::UpdateErrorCodeMessageAndDetailsBody,
            guest_endpoints::GuestUserBody,
            guest_role_endpoints::CreateGuestRoleBody,
            role_endpoints::CreateRoleBody,
        ),
    ),
    tags(
        (
            name = "manager",
            description = "Manager Users management endpoints."
        )
    ),
)]
pub struct ApiDoc;

// ? ---------------------------------------------------------------------------
// ? This module contained the results-expert endpoints
// ? ---------------------------------------------------------------------------

pub mod error_code_endpoints {

    use crate::{
        endpoints::shared::PaginationParams,
        modules::{
            ErrorCodeDeletionModule, ErrorCodeFetchingModule,
            ErrorCodeRegistrationModule, ErrorCodeUpdatingModule,
        },
    };

    use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
    use actix_web_httpauth::extractors::bearer::Config;
    use clean_base::entities::{FetchManyResponseKind, FetchResponseKind};
    use myc_core::{
        domain::{
            dtos::native_error_codes::NativeErrorCodes,
            entities::{
                ErrorCodeDeletion, ErrorCodeFetching, ErrorCodeRegistration,
                ErrorCodeUpdating,
            },
        },
        use_cases::gateway::error_codes::{
            delete_error_code, get_error_code, list_error_codes,
            register_error_code, update_error_code_message_and_details,
        },
    };
    use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::{IntoParams, ToSchema};

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/error-codes")
                .app_data(Config::default())
                .service(register_error_code_url)
                .service(list_error_codes_url)
                .service(get_error_code_url)
                .service(update_error_code_message_and_details_url)
                .service(delete_error_code_url),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateErrorCodeBody {
        prefix: String,
        message: String,
        details: Option<String>,
        is_internal: bool,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct ListErrorCodesParams {
        prefix: Option<String>,
        code: Option<i32>,
        is_internal: Option<bool>,
    }

    #[derive(Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateErrorCodeMessageAndDetailsBody {
        message: String,
        details: Option<String>,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    // ? -----------------------------------------------------------------------

    /// Register a new error code.
    ///
    /// This action is restricted to manager users.
    #[utoipa::path(
        post,
        context_path = "/myc/managers/error-codes",
        request_body = CreateErrorCodeBody,
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
                description = "Error code already exists.",
                body = JsonError,
            ),
            (
                status = 201,
                description = "Error code created.",
                body = ErrorCode,
            ),
        ),
    )]
    #[post("/")]
    pub async fn register_error_code_url(
        body: web::Json<CreateErrorCodeBody>,
        profile: MyceliumProfileData,
        error_code_registration_repo: Inject<
            ErrorCodeRegistrationModule,
            dyn ErrorCodeRegistration,
        >,
    ) -> impl Responder {
        match register_error_code(
            profile.to_profile(),
            body.prefix.to_owned(),
            body.message.to_owned(),
            body.details.to_owned(),
            body.is_internal.to_owned(),
            Box::new(&*error_code_registration_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError().json(
                JsonError::new(err.to_string())
                    .with_code(err.code().to_string()),
            ),
            Ok(account) => HttpResponse::Created().json(account),
        }
    }

    /// List available error codes.
    ///
    /// List accounts with pagination. The `records` field contains a vector of
    /// [`ErrorCode`] model.
    ///
    #[utoipa::path(
        get,
        context_path = "/myc/managers/error-codes",
        params(
            ListErrorCodesParams,
            PaginationParams,
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
                body = [ErrorCode],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_error_codes_url(
        info: web::Query<ListErrorCodesParams>,
        page: web::Query<PaginationParams>,
        profile: MyceliumProfileData,
        error_code_fetching_repo: Inject<
            ErrorCodeFetchingModule,
            dyn ErrorCodeFetching,
        >,
    ) -> impl Responder {
        match list_error_codes(
            profile.to_profile(),
            info.prefix.to_owned(),
            info.code.to_owned(),
            info.is_internal.to_owned(),
            page.page_size.to_owned(),
            page.skip.to_owned(),
            Box::new(&*error_code_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                FetchManyResponseKind::NotFound => {
                    HttpResponse::NotFound().finish()
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
        get,
        context_path = "/myc/managers/error-codes",
        params(
            ("prefix" = String, Path, description = "The error prefix."),
            ("code" = i32, Path, description = "The error code."),
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
    #[get("/prefix/{account}/code/{code}")]
    pub async fn get_error_code_url(
        path: web::Path<(String, i32)>,
        profile: MyceliumProfileData,
        error_code_fetching_repo: Inject<
            ErrorCodeFetchingModule,
            dyn ErrorCodeFetching,
        >,
    ) -> impl Responder {
        let (prefix, code) = path.into_inner();

        match get_error_code(
            profile.to_profile(),
            prefix.to_owned(),
            code.to_owned(),
            Box::new(&*error_code_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                FetchResponseKind::NotFound(_) => HttpResponse::NotFound()
                    .json(JsonError::new(format!(
                        "Error code not found: {prefix}-{code}"
                    ))),
                FetchResponseKind::Found(accounts) => {
                    HttpResponse::Ok().json(accounts)
                }
            },
        }
    }

    #[utoipa::path(
        patch,
        context_path = "/myc/managers/error-codes",
        params(
            ("prefix" = String, Path, description = "The error prefix."),
            ("code" = i32, Path, description = "The error code."),
        ),
        request_body = UpdateErrorCodeMessageAndDetailsBody,
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
                description = "Error code not updated.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Error code updated.",
                body = Account,
            ),
        ),
    )]
    #[patch("/prefix/{account}/code/{code}")]
    pub async fn update_error_code_message_and_details_url(
        path: web::Path<(String, i32)>,
        body: web::Json<UpdateErrorCodeMessageAndDetailsBody>,
        profile: MyceliumProfileData,
        error_code_fetching_repo: Inject<
            ErrorCodeFetchingModule,
            dyn ErrorCodeFetching,
        >,
        error_code_updating_repo: Inject<
            ErrorCodeUpdatingModule,
            dyn ErrorCodeUpdating,
        >,
    ) -> impl Responder {
        let (prefix, code) = path.into_inner();

        match update_error_code_message_and_details(
            profile.to_profile(),
            prefix,
            code,
            body.message.to_owned(),
            body.details.to_owned(),
            Box::new(&*error_code_fetching_repo),
            Box::new(&*error_code_updating_repo),
        )
        .await
        {
            Err(err) => {
                let target_msg = NativeErrorCodes::MYC00005.as_str();
                if err.is_in(vec![target_msg]) {
                    return HttpResponse::BadRequest().json(
                        JsonError::new(err.to_string())
                            .with_code_str(target_msg),
                    );
                }

                HttpResponse::InternalServerError()
                    .json(JsonError::new(err.to_string()))
            }
            Ok(res) => HttpResponse::Accepted().json(res),
        }
    }

    #[utoipa::path(
        delete,
        context_path = "/myc/managers/error-codes",
        params(
            ("prefix" = String, Path, description = "The error prefix."),
            ("code" = i32, Path, description = "The error code."),
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
                description = "Error code not deleted.",
                body = JsonError,
            ),
            (
                status = 204,
                description = "Error code deleted.",
            ),
        ),
    )]
    #[delete("/prefix/{account}/code/{code}")]
    pub async fn delete_error_code_url(
        path: web::Path<(String, i32)>,
        profile: MyceliumProfileData,
        error_code_deletion_repo: Inject<
            ErrorCodeDeletionModule,
            dyn ErrorCodeDeletion,
        >,
    ) -> impl Responder {
        let (prefix, code) = path.into_inner();

        match delete_error_code(
            profile.to_profile(),
            prefix,
            code,
            Box::new(&*error_code_deletion_repo),
        )
        .await
        {
            Err(err) => {
                let target_msg = NativeErrorCodes::MYC00007.as_str();

                if err.is_in(vec![target_msg]) {
                    return HttpResponse::Forbidden().json(
                        JsonError::new(err.to_string())
                            .with_code_str(target_msg),
                    );
                }

                HttpResponse::InternalServerError()
                    .json(JsonError::new(err.to_string()))
            }
            Ok(_) => HttpResponse::NoContent().finish(),
        }
    }
}

pub mod account_endpoints {

    use crate::{
        endpoints::shared::PaginationParams,
        modules::{
            AccountFetchingModule, AccountRegistrationModule,
            AccountTypeRegistrationModule, AccountUpdatingModule,
            UserRegistrationModule,
        },
    };

    use actix_web::{get, patch, post, web, HttpResponse, Responder};
    use actix_web_httpauth::extractors::bearer::Config;
    use clean_base::entities::{
        FetchManyResponseKind, FetchResponseKind, UpdatingResponseKind,
    };
    use myc_core::{
        domain::{
            dtos::{
                account::VerboseStatus, native_error_codes::NativeErrorCodes,
            },
            entities::{
                AccountFetching, AccountRegistration, AccountTypeRegistration,
                AccountUpdating, UserRegistration,
            },
        },
        use_cases::roles::managers::account::{
            change_account_activation_status, change_account_approval_status,
            change_account_archival_status, create_subscription_account,
            get_account_details, list_accounts_by_type,
        },
    };
    use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::{IntoParams, ToSchema};
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/accounts")
                .app_data(Config::default())
                .service(create_subscription_account_url)
                .service(list_accounts_by_type_url)
                .service(get_account_details_url)
                .service(approve_account_url)
                .service(disapprove_account_url)
                .service(activate_account_url)
                .service(deactivate_account_url)
                .service(archive_account_url)
                .service(unarchive_account_url),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateSubscriptionAccountBody {
        pub email: String,
        pub account_name: String,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct ListSubscriptionAccountParams {
        term: Option<String>,
        is_subscription: Option<bool>,
        is_owner_active: Option<bool>,
        status: Option<VerboseStatus>,
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
        context_path = "/myc/managers/accounts",
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
                body = Account,
            ),
        ),
    )]
    #[post("/")]
    pub async fn create_subscription_account_url(
        body: web::Json<CreateSubscriptionAccountBody>,
        profile: MyceliumProfileData,
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
        match create_subscription_account(
            profile.to_profile(),
            body.email.to_owned(),
            body.account_name.to_owned(),
            Box::new(&*user_registration_repo),
            Box::new(&*account_type_registration_repo),
            Box::new(&*account_registration_repo),
        )
        .await
        {
            Err(err) => {
                let code_string = err.code().to_string();

                if err.is_in(vec![
                    NativeErrorCodes::MYC00002.as_str(),
                    NativeErrorCodes::MYC00003.as_str(),
                ]) {
                    return HttpResponse::BadRequest().json(
                        JsonError::new(err.to_string()).with_code(code_string),
                    );
                }

                HttpResponse::InternalServerError().json(
                    JsonError::new(err.to_string()).with_code(code_string),
                )
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
        context_path = "/myc/managers/accounts",
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
                status = 404,
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
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
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
                        return HttpResponse::NotFound()
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
                    HttpResponse::NotFound().finish()
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
        context_path = "/myc/managers/accounts",
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
                status = 404,
                description = "Not found.",
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
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
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
                FetchResponseKind::NotFound(id) => HttpResponse::NotFound()
                    .json(JsonError::new(id.unwrap().to_string())),
                FetchResponseKind::Found(accounts) => {
                    HttpResponse::Ok().json(accounts)
                }
            },
        }
    }

    /// Approve account after creation
    ///
    /// New accounts should be approved after has permissions to perform
    /// operation on the system. These endpoint should approve such account.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/accounts",
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
                description = "Account not approved.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Account approved.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/approve")]
    pub async fn approve_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match change_account_approval_status(
            profile.to_profile(),
            path.to_owned(),
            true,
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

    /// Disapprove account after creation
    ///
    /// Also approved account should be disapproved at any time. These endpoint
    /// work for this.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/accounts",
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
                description = "Account not disapproved.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Account disapproved.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/disapprove")]
    pub async fn disapprove_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match change_account_approval_status(
            profile.to_profile(),
            path.to_owned(),
            false,
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

    /// Activate account
    ///
    /// Any account could be activated and deactivated. This action turn an
    /// account active.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/accounts",
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
                description = "Account not activated.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Account activated.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/activate")]
    pub async fn activate_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match change_account_activation_status(
            profile.to_profile(),
            path.to_owned(),
            true,
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

    /// Deactivate account
    ///
    /// Any account could be activated and deactivated. This action turn an
    /// account deactivated.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/accounts",
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
                description = "Account not activated.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Account activated.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/deactivate")]
    pub async fn deactivate_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match change_account_activation_status(
            profile.to_profile(),
            path.to_owned(),
            false,
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

    /// Archive account
    ///
    /// Set target account as archived.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/accounts",
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
                description = "Account not activated.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Account activated.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/archive")]
    pub async fn archive_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match change_account_archival_status(
            profile.to_profile(),
            path.to_owned(),
            true,
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

    /// Unarchive account
    ///
    /// Set target account as un-archived.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/accounts",
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
                description = "Account not activated.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Account activated.",
                body = Account,
            ),
        ),
    )]
    #[patch("/{account}/unarchive")]
    pub async fn unarchive_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match change_account_archival_status(
            profile.to_profile(),
            path.to_owned(),
            false,
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

pub mod guest_endpoints {

    use crate::modules::{
        AccountFetchingModule, GuestUserDeletionModule,
        GuestUserFetchingModule, GuestUserOnAccountUpdatingModule,
        GuestUserRegistrationModule, LicensedResourcesFetchingModule,
        MessageSendingModule,
    };

    use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
    use clean_base::entities::{
        DeletionResponseKind, FetchManyResponseKind, GetOrCreateResponseKind,
        UpdatingResponseKind,
    };
    use myc_core::{
        domain::{
            dtos::email::Email,
            entities::{
                AccountFetching, GuestUserDeletion, GuestUserFetching,
                GuestUserOnAccountUpdating, GuestUserRegistration,
                LicensedResourcesFetching, MessageSending,
            },
        },
        use_cases::roles::managers::guest::{
            guest_user, list_guest_on_subscription_account,
            list_licensed_accounts_of_email, uninvite_guest,
            update_user_guest_role,
        },
    };
    use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::{IntoParams, ToSchema};
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/guests")
                .service(list_licensed_accounts_of_email_url)
                .service(guest_user_url)
                .service(uninvite_guest_url)
                .service(update_user_guest_role_url)
                .service(list_guest_on_subscription_account_url),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, ToSchema, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct GuestUserBody {
        pub email: String,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateUserGuestRoleParams {
        pub new_guest_role_id: Uuid,
    }

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Guest
    //
    // ? -----------------------------------------------------------------------

    /// List subscription accounts which email was guest
    #[utoipa::path(
        get,
        context_path = "/myc/managers/guests",
        params(
            GuestUserBody
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
                body = [LicensedResources],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_licensed_accounts_of_email_url(
        info: web::Query<GuestUserBody>,
        profile: MyceliumProfileData,
        licensed_resources_fetching_repo: Inject<
            LicensedResourcesFetchingModule,
            dyn LicensedResourcesFetching,
        >,
    ) -> impl Responder {
        let email = match Email::from_string(info.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest()
                    .json(JsonError::new(format!("Invalid email: {err}")))
            }
            Ok(res) => res,
        };

        match list_licensed_accounts_of_email(
            profile.to_profile(),
            email.to_owned(),
            Box::new(&*licensed_resources_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                FetchManyResponseKind::NotFound => HttpResponse::NotFound()
                    .json(JsonError::new(format!(
                        "Account ({}) was not guest to any subscription account.",
                        email.get_email()
                    ))),
                FetchManyResponseKind::Found(guests) => {
                    HttpResponse::Ok().json(guests)
                }
                FetchManyResponseKind::FoundPaginated(guests) => {
                    HttpResponse::Ok().json(guests)
                }
            },
        }
    }

    /// Guest a user to work on account.
    ///
    /// This action gives the ability of the target account (specified through
    /// the `account` argument) to perform actions specified in the `role`
    /// path argument.
    #[utoipa::path(
        post,
        context_path = "/myc/managers/guests",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            ("role" = Uuid, Path, description = "The guest-role unique id."),
        ),
        request_body = GuestUserBody,
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
        body: web::Json<GuestUserBody>,
        profile: MyceliumProfileData,
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
        let (account_id, role_id) = path.to_owned();

        let email = match Email::from_string(body.email.to_owned()) {
            Err(err) => {
                return HttpResponse::BadRequest()
                    .json(JsonError::new(err.to_string()))
            }
            Ok(res) => res,
        };

        match guest_user(
            profile.to_profile(),
            email,
            role_id,
            account_id,
            Box::new(&*account_fetching_repo),
            Box::new(&*guest_registration_repo),
            Box::new(&*message_sending_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
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

    /// Update guest-role of a single user.
    ///
    /// This action gives the ability of the target account (specified through
    /// the `account` argument) to replace the current specified `role` by the
    /// new role.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/guests",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            ("role" = Uuid, Path, description = "The guest-role unique id."),
            UpdateUserGuestRoleParams,
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
                description = "Guesting done.",
                body = GuestUser,
            ),
        ),
    )]
    #[patch("/account/{account}/role/{role}")]
    pub async fn update_user_guest_role_url(
        path: web::Path<(Uuid, Uuid)>,
        info: web::Query<UpdateUserGuestRoleParams>,
        profile: MyceliumProfileData,
        guest_user_on_account_updating_repo: Inject<
            GuestUserOnAccountUpdatingModule,
            dyn GuestUserOnAccountUpdating,
        >,
    ) -> impl Responder {
        let (account_id, role_id) = path.to_owned();

        match update_user_guest_role(
            profile.to_profile(),
            role_id,
            account_id,
            info.new_guest_role_id.to_owned(),
            Box::new(&*guest_user_on_account_updating_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                UpdatingResponseKind::NotUpdated(_, msg) => {
                    HttpResponse::Ok().json(JsonError::new(msg))
                }
                UpdatingResponseKind::Updated(guest) => {
                    HttpResponse::Created().json(guest)
                }
            },
        }
    }

    /// Uninvite user to perform a role to account
    #[utoipa::path(
        delete,
        context_path = "/myc/managers/guests",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            ("role" = Uuid, Path, description = "The guest-role unique id."),
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
                description = "Guest User not uninvited.",
                body = JsonError,
            ),
            (
                status = 204,
                description = "Guest User uninvited.",
            ),
        ),
    )]
    #[delete("/account/{account}/role/{role}")]
    pub async fn uninvite_guest_url(
        path: web::Path<(Uuid, Uuid)>,
        info: web::Query<GuestUserBody>,
        profile: MyceliumProfileData,
        guest_user_deletion_repo: Inject<
            GuestUserDeletionModule,
            dyn GuestUserDeletion,
        >,
    ) -> impl Responder {
        let (account_id, role_id) = path.to_owned();

        match uninvite_guest(
            profile.to_profile(),
            account_id,
            role_id,
            info.email.to_owned(),
            Box::new(&*guest_user_deletion_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                DeletionResponseKind::NotDeleted(_, msg) => {
                    HttpResponse::Conflict().json(JsonError::new(msg))
                }
                DeletionResponseKind::Deleted => {
                    HttpResponse::NoContent().finish()
                }
            },
        }
    }

    /// List guest accounts related to a subscription account
    ///
    /// This action fetches all non-subscription accounts related to the
    /// informed subscription account.
    #[utoipa::path(
        get,
        context_path = "/myc/managers/guests",
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
                status = 404,
                description = "Not found.",
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
                status = 200,
                description = "Fetching success.",
                body = GuestUser,
            ),
        ),
    )]
    #[get("/account/{account}")]
    pub async fn list_guest_on_subscription_account_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        guest_user_fetching_repo: Inject<
            GuestUserFetchingModule,
            dyn GuestUserFetching,
        >,
    ) -> impl Responder {
        let account_id = path.to_owned();

        match list_guest_on_subscription_account(
            profile.to_profile(),
            account_id.to_owned(),
            Box::new(&*account_fetching_repo),
            Box::new(&*guest_user_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                FetchManyResponseKind::NotFound => HttpResponse::NotFound()
                    .json(JsonError::new(format!(
                        "Account ({}) has no associated guests",
                        account_id
                    ))),
                FetchManyResponseKind::Found(guests) => {
                    HttpResponse::Ok().json(guests)
                }
                FetchManyResponseKind::FoundPaginated(guests) => {
                    HttpResponse::Ok().json(guests)
                }
            },
        }
    }
}

pub mod guest_role_endpoints {

    use crate::modules::{
        GuestRoleDeletionModule, GuestRoleFetchingModule,
        GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    };

    use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
    use clean_base::entities::{
        DeletionResponseKind, FetchManyResponseKind, GetOrCreateResponseKind,
        UpdatingResponseKind,
    };
    use myc_core::{
        domain::{
            dtos::guest::PermissionsType,
            entities::{
                GuestRoleDeletion, GuestRoleFetching, GuestRoleRegistration,
                GuestRoleUpdating,
            },
        },
        use_cases::roles::managers::guest_role::{
            create_guest_role, delete_guest_role, list_guest_roles,
            update_guest_role_name_and_description,
            update_guest_role_permissions, ActionType,
        },
    };
    use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::{IntoParams, ToSchema};
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/guest-roles")
                .service(crate_guest_role_url)
                .service(list_guest_roles_url)
                .service(delete_guest_role_url)
                .service(update_guest_role_name_and_description_url)
                .service(update_guest_role_permissions_url),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateGuestRoleBody {
        pub name: String,
        pub description: String,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct ListGuestRolesParams {
        pub name: Option<String>,
        pub role_id: Option<Uuid>,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateGuestRoleNameAndDescriptionParams {
        pub name: Option<String>,
        pub description: Option<String>,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct UpdateGuestRolePermissionsParams {
        pub permission: PermissionsType,
        pub action_type: ActionType,
    }

    /// Create Guest Role
    ///
    /// Guest Roles provide permissions to simple Roles.
    #[utoipa::path(
        post,
        context_path = "/myc/managers/guest-roles",
        params(
            ("role" = Uuid, Path, description = "The guest-role primary key."),
        ),
        request_body = CreateGuestRoleBody,
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
                description = "Guest Role created.",
                body = GuestRole,
            ),
            (
                status = 200,
                description = "Guest Role already exists.",
                body = GuestRole,
            ),
        ),
    )]
    #[post("/{role}/")]
    pub async fn crate_guest_role_url(
        path: web::Path<Uuid>,
        json: web::Json<CreateGuestRoleBody>,
        profile: MyceliumProfileData,
        role_registration_repo: Inject<
            GuestRoleRegistrationModule,
            dyn GuestRoleRegistration,
        >,
    ) -> impl Responder {
        match create_guest_role(
            profile.to_profile(),
            json.name.to_owned(),
            json.description.to_owned(),
            path.to_owned(),
            None,
            Box::new(&*role_registration_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
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

    /// List Roles
    #[utoipa::path(
        get,
        context_path = "/myc/managers/guest-roles",
        params(
            ListGuestRolesParams,
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
                description = "Success.",
                body = [Role],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_guest_roles_url(
        info: web::Query<ListGuestRolesParams>,
        profile: MyceliumProfileData,
        guest_role_fetching_repo: Inject<
            GuestRoleFetchingModule,
            dyn GuestRoleFetching,
        >,
    ) -> impl Responder {
        match list_guest_roles(
            profile.to_profile(),
            info.name.to_owned(),
            info.role_id.to_owned(),
            Box::new(&*guest_role_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                FetchManyResponseKind::NotFound => {
                    HttpResponse::NotFound().finish()
                }
                FetchManyResponseKind::Found(roles) => {
                    HttpResponse::Ok().json(roles)
                }
                FetchManyResponseKind::FoundPaginated(roles) => {
                    HttpResponse::Ok().json(roles)
                }
            },
        }
    }

    /// Delete Guest Role
    ///
    /// Delete a single guest role.
    #[utoipa::path(
        delete,
        context_path = "/myc/managers/guest-roles",
        params(
            ("role" = Uuid, Path, description = "The guest-role primary key."),
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
                description = "Guest Role not deleted.",
                body = JsonError,
            ),
            (
                status = 204,
                description = "Guest Role deleted.",
            ),
        ),
    )]
    #[delete("/{role}/delete")]
    pub async fn delete_guest_role_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        role_deletion_repo: Inject<
            GuestRoleDeletionModule,
            dyn GuestRoleDeletion,
        >,
    ) -> impl Responder {
        match delete_guest_role(
            profile.to_profile(),
            path.to_owned(),
            Box::new(&*role_deletion_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                DeletionResponseKind::NotDeleted(_, msg) => {
                    HttpResponse::BadRequest().json(JsonError::new(msg))
                }
                DeletionResponseKind::Deleted => {
                    HttpResponse::NoContent().finish()
                }
            },
        }
    }

    /// Partial Update Guest Role
    ///
    /// Update name and description of a single Guest Role.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/guest-roles",
        params(
            ("role" = Uuid, Path, description = "The guest-role primary key."),
            UpdateGuestRoleNameAndDescriptionParams,
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
                description = "Guest Role not deleted.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Guest Role updated.",
                body = GuestRole,
            ),
        ),
    )]
    #[patch("/{role}/update-name-and-description")]
    pub async fn update_guest_role_name_and_description_url(
        path: web::Path<Uuid>,
        info: web::Query<UpdateGuestRoleNameAndDescriptionParams>,
        profile: MyceliumProfileData,
        role_fetching_repo: Inject<
            GuestRoleFetchingModule,
            dyn GuestRoleFetching,
        >,
        role_updating_repo: Inject<
            GuestRoleUpdatingModule,
            dyn GuestRoleUpdating,
        >,
    ) -> impl Responder {
        match update_guest_role_name_and_description(
            profile.to_profile(),
            info.name.to_owned(),
            info.description.to_owned(),
            path.to_owned(),
            Box::new(&*role_fetching_repo),
            Box::new(&*role_updating_repo),
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

    /// Change permissions of Guest Role
    ///
    /// Upgrade or Downgrade permissions of Guest Role.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/guest-roles",
        params(
            ("role" = Uuid, Path, description = "The guest-role primary key."),
            UpdateGuestRolePermissionsParams,
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
                description = "Guest Role not deleted.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Guest Role updated.",
                body = GuestRole,
            ),
        ),
    )]
    #[patch("/{role}/update-permissions")]
    pub async fn update_guest_role_permissions_url(
        path: web::Path<Uuid>,
        info: web::Query<UpdateGuestRolePermissionsParams>,
        profile: MyceliumProfileData,
        role_fetching_repo: Inject<
            GuestRoleFetchingModule,
            dyn GuestRoleFetching,
        >,
        role_updating_repo: Inject<
            GuestRoleUpdatingModule,
            dyn GuestRoleUpdating,
        >,
    ) -> impl Responder {
        match update_guest_role_permissions(
            profile.to_profile(),
            path.to_owned(),
            info.permission.to_owned(),
            info.action_type.to_owned(),
            Box::new(&*role_fetching_repo),
            Box::new(&*role_updating_repo),
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

    // ? -----------------------------------------------------------------------
    // ? Define API paths
    //
    // Role
    //
    // ? -----------------------------------------------------------------------
}

pub mod role_endpoints {

    use crate::modules::{
        RoleDeletionModule, RoleFetchingModule, RoleRegistrationModule,
        RoleUpdatingModule,
    };

    use actix_web::{delete, get, patch, post, web, HttpResponse, Responder};
    use clean_base::entities::{
        DeletionResponseKind, FetchManyResponseKind, GetOrCreateResponseKind,
        UpdatingResponseKind,
    };
    use myc_core::{
        domain::entities::{
            RoleDeletion, RoleFetching, RoleRegistration, RoleUpdating,
        },
        use_cases::roles::managers::role::{
            create_role, delete_role, list_roles,
            update_role_name_and_description,
        },
    };
    use myc_http_tools::{middleware::MyceliumProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::{IntoParams, ToSchema};
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/roles")
                .service(crate_role_url)
                .service(list_roles_url)
                .service(delete_role_url)
                .service(update_role_name_and_description_url),
        );
    }

    // ? -----------------------------------------------------------------------
    // ? Define API structs
    // ? -----------------------------------------------------------------------

    #[derive(Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateRoleBody {
        pub name: String,
        pub description: String,
    }

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct ListRolesParams {
        pub name: Option<String>,
    }

    /// Create Role
    ///
    /// Roles are used to build Guest Role elements.
    #[utoipa::path(
        post,
        context_path = "/myc/managers/roles",
        request_body = CreateRoleBody,
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
                description = "Role created.",
                body = Role,
            ),
            (
                status = 200,
                description = "Role already exists.",
                body = Role,
            ),
        ),
    )]
    #[post("/")]
    pub async fn crate_role_url(
        body: web::Json<CreateRoleBody>,
        profile: MyceliumProfileData,
        role_registration_repo: Inject<
            RoleRegistrationModule,
            dyn RoleRegistration,
        >,
    ) -> impl Responder {
        match create_role(
            profile.to_profile(),
            body.name.to_owned(),
            body.description.to_owned(),
            Box::new(&*role_registration_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
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

    /// List Roles
    #[utoipa::path(
        get,
        context_path = "/myc/managers/roles",
        params(
            ListRolesParams,
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
                status = 401,
                description = "Unauthorized.",
                body = JsonError,
            ),
            (
                status = 403,
                description = "Forbidden.",
                body = JsonError,
            ),
            (
                status = 200,
                description = "Success.",
                body = [Role],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_roles_url(
        info: web::Query<ListRolesParams>,
        profile: MyceliumProfileData,
        roles_fetching_repo: Inject<RoleFetchingModule, dyn RoleFetching>,
    ) -> impl Responder {
        let name = info.name.to_owned();

        match list_roles(
            profile.to_profile(),
            name.to_owned(),
            Box::new(&*roles_fetching_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                FetchManyResponseKind::NotFound => HttpResponse::NoContent()
                    .json(JsonError::new(name.unwrap_or("".to_string()))),
                FetchManyResponseKind::Found(roles) => {
                    HttpResponse::Ok().json(roles)
                }
                FetchManyResponseKind::FoundPaginated(roles) => {
                    HttpResponse::Ok().json(roles)
                }
            },
        }
    }

    /// Delete Role
    ///
    /// Delete a single role.
    #[utoipa::path(
        delete,
        context_path = "/myc/managers/roles",
        params(
            ("role" = Uuid, Path, description = "The role primary key."),
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = JsonError,
            ),
            (
                status = 400,
                description = "Role not deleted.",
                body = JsonError,
            ),
            (
                status = 401,
                description = "Unauthorized.",
                body = JsonError,
            ),
            (
                status = 403,
                description = "Forbidden.",
                body = JsonError,
            ),
            (
                status = 204,
                description = "Role deleted.",
            ),
        ),
    )]
    #[delete("/{role}/delete")]
    pub async fn delete_role_url(
        path: web::Path<Uuid>,
        profile: MyceliumProfileData,
        role_deletion_repo: Inject<RoleDeletionModule, dyn RoleDeletion>,
    ) -> impl Responder {
        match delete_role(
            profile.to_profile(),
            path.to_owned(),
            Box::new(&*role_deletion_repo),
        )
        .await
        {
            Err(err) => HttpResponse::InternalServerError()
                .json(JsonError::new(err.to_string())),
            Ok(res) => match res {
                DeletionResponseKind::NotDeleted(_, msg) => {
                    HttpResponse::BadRequest().json(JsonError::new(msg))
                }
                DeletionResponseKind::Deleted => {
                    HttpResponse::NoContent().finish()
                }
            },
        }
    }

    /// Partial Update Role
    ///
    /// Update name and description of a single Role.
    #[utoipa::path(
        patch,
        context_path = "/myc/managers/roles",
        params(
            ("role" = Uuid, Path, description = "The role primary key."),
        ),
        request_body = CreateRoleBody,
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
                description = "Guest Role not deleted.",
                body = JsonError,
            ),
            (
                status = 202,
                description = "Guest Role updated.",
                body = Role,
            ),
        ),
    )]
    #[patch("/{role}/update-name-and-description")]
    pub async fn update_role_name_and_description_url(
        path: web::Path<Uuid>,
        body: web::Json<CreateRoleBody>,
        profile: MyceliumProfileData,
        role_fetching_repo: Inject<RoleFetchingModule, dyn RoleFetching>,
        role_updating_repo: Inject<RoleUpdatingModule, dyn RoleUpdating>,
    ) -> impl Responder {
        match update_role_name_and_description(
            profile.to_profile(),
            path.to_owned(),
            body.name.to_owned(),
            body.description.to_owned(),
            Box::new(&*role_fetching_repo),
            Box::new(&*role_updating_repo),
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
