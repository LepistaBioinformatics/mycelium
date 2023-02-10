use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::{
    domain::dtos::{
        account::{Account, AccountType},
        email::Email,
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
        account_endpoints::list_subscription_accounts_url,
        account_endpoints::get_subscription_account_details_url,
        account_endpoints::approve_account_url,
        account_endpoints::activate_account_url,
        account_endpoints::deactivate_account_url,
        guest_endpoints::guest_user_url,
        guest_endpoints::list_guest_on_subscription_account_url,
        guest_role_endpoints::crate_guest_role_url,
        guest_role_endpoints::delete_guest_role_url,
        guest_role_endpoints::update_guest_role_name_and_description_url,
        guest_role_endpoints::update_guest_role_permissions_url,
        role_endpoints::crate_role_url,
        role_endpoints::delete_role_url,
        role_endpoints::update_role_name_and_description_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account,
            AccountType,
            ActionType,
            Email,
            GuestUser,
            GuestRole,
            JsonError,
            LicensedResources,
            PermissionsType,
            Profile,
            Role,
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

pub mod account_endpoints {

    use crate::modules::{
        AccountFetchingModule, AccountRegistrationModule,
        AccountTypeRegistrationModule, AccountUpdatingModule,
        UserRegistrationModule,
    };

    use actix_web::{get, patch, post, web, HttpResponse, Responder};
    use actix_web_httpauth::extractors::bearer::Config;
    use clean_base::entities::default_response::{
        FetchManyResponseKind, FetchResponseKind, GetOrCreateResponseKind,
        UpdatingResponseKind,
    };
    use myc_core::{
        domain::entities::{
            AccountFetching, AccountRegistration, AccountTypeRegistration,
            AccountUpdating, UserRegistration,
        },
        use_cases::roles::{
            managers::account::{
                create_subscription_account, get_subscription_account_details,
                list_subscription_accounts,
            },
            shared::account::{
                approve_account, change_account_activation_status,
            },
        },
    };
    use myc_http_tools::{middleware::ProfileData, utils::JsonError};
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
                .app_data(Config::default())
                .service(create_subscription_account_url)
                .service(list_subscription_accounts_url)
                .service(get_subscription_account_details_url)
                .service(approve_account_url)
                .service(activate_account_url)
                .service(deactivate_account_url),
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
    pub struct ListSubscriptionAccountParams {
        pub name: Option<String>,
        pub is_active: Option<bool>,
        pub is_checked: Option<bool>,
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
        path = "/myc/managers/accounts/",
        params(
            CreateSubscriptionAccountParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = JsonError,
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
        profile: ProfileData,
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
            info.email.to_owned(),
            info.account_name.to_owned(),
            Box::new(&*user_registration_repo),
            Box::new(&*account_type_registration_repo),
            Box::new(&*account_registration_repo),
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

    /// List Subscription Accounts
    ///
    /// Get a filtered (or not) list of accounts.
    #[utoipa::path(
        get,
        path = "/myc/managers/accounts/",
        params(
            ListSubscriptionAccountParams,
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
                status = 200,
                description = "Fetching success.",
                body = [Account],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_subscription_accounts_url(
        info: web::Query<ListSubscriptionAccountParams>,
        profile: ProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_type_registration_repo: Inject<
            AccountTypeRegistrationModule,
            dyn AccountTypeRegistration,
        >,
    ) -> impl Responder {
        match list_subscription_accounts(
            profile.to_profile(),
            info.name.to_owned(),
            info.is_active.to_owned(),
            info.is_checked.to_owned(),
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
            },
        }
    }

    /// Get Subscription Account
    ///
    /// Get a single subscription account.
    #[utoipa::path(
        get,
        path = "/myc/managers/accounts/{account}",
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
                status = 200,
                description = "Fetching success.",
                body = Account,
            ),
        ),
    )]
    #[get("/{account}")]
    pub async fn get_subscription_account_details_url(
        path: web::Path<Uuid>,
        profile: ProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
    ) -> impl Responder {
        match get_subscription_account_details(
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
        path = "/myc/managers/accounts/{account}/approve",
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
        profile: ProfileData,
        account_fetching_repo: Inject<
            AccountFetchingModule,
            dyn AccountFetching,
        >,
        account_updating_repo: Inject<
            AccountUpdatingModule,
            dyn AccountUpdating,
        >,
    ) -> impl Responder {
        match approve_account(
            profile.to_profile(),
            path.to_owned(),
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
        path = "/myc/managers/accounts/{account}/activate",
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
        profile: ProfileData,
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
        path = "/myc/managers/accounts/{account}/deactivate",
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
        profile: ProfileData,
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
}

pub mod guest_endpoints {

    use crate::modules::{
        AccountFetchingModule, GuestUserFetchingModule,
        GuestUserRegistrationModule, MessageSendingModule,
    };

    use actix_web::{get, post, web, HttpResponse, Responder};
    use clean_base::entities::default_response::{
        FetchManyResponseKind, GetOrCreateResponseKind,
    };
    use myc_core::{
        domain::{
            dtos::email::Email,
            entities::{
                AccountFetching, GuestUserFetching, GuestUserRegistration,
                MessageSending,
            },
        },
        use_cases::roles::managers::guest::{
            guest_user, list_guest_on_subscription_account,
        },
    };
    use myc_http_tools::{middleware::ProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
    use uuid::Uuid;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(
            web::scope("/guests")
                .service(guest_user_url)
                .service(list_guest_on_subscription_account_url),
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
        path = "/myc/managers/guests/account/{account}/role/{role}",
        params(
            ("account" = Uuid, Path, description = "The account primary key."),
            ("role" = Uuid, Path, description = "The guest-role unique id."),
            GuestUserParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
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
        info: web::Query<GuestUserParams>,
        profile: ProfileData,
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

        let email = match Email::from_string(info.email.to_owned()) {
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

    /// List guest accounts related to a subscription account
    ///
    /// This action fetches all non-subscription accounts related to the
    /// informed subscription account.
    #[utoipa::path(
        get,
        path = "/myc/managers/guests/account/{account}",
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
                status = 200,
                description = "Fetching success.",
                body = GuestUser,
            ),
        ),
    )]
    #[get("/account/{account}")]
    pub async fn list_guest_on_subscription_account_url(
        path: web::Path<Uuid>,
        profile: ProfileData,
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
                FetchManyResponseKind::Found(guest) => {
                    HttpResponse::Ok().json(guest)
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
    use clean_base::entities::default_response::{
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
    use myc_http_tools::{middleware::ProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
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

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateGuestRoleParams {
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
        path = "/myc/managers/guest-roles/{role}/",
        params(
            ("role" = Uuid, Path, description = "The guest-role primary key."),
            CreateGuestRoleParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
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
        info: web::Query<CreateGuestRoleParams>,
        profile: ProfileData,
        role_registration_repo: Inject<
            GuestRoleRegistrationModule,
            dyn GuestRoleRegistration,
        >,
    ) -> impl Responder {
        match create_guest_role(
            profile.to_profile(),
            info.name.to_owned(),
            info.description.to_owned(),
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
        path = "/myc/managers/guest-roles/",
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
                status = 200,
                description = "Success.",
                body = [Role],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_guest_roles_url(
        info: web::Query<ListGuestRolesParams>,
        profile: ProfileData,
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
            },
        }
    }

    /// Delete Guest Role
    ///
    /// Delete a single guest role.
    #[utoipa::path(
        delete,
        path = "/myc/managers/guest-roles/{role}/delete",
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
        profile: ProfileData,
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
        path = "/myc/managers/guest-roles/{role}/update-name-and-description",
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
        profile: ProfileData,
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
        path = "/myc/managers/guest-roles/{role}/update-permissions",
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
        profile: ProfileData,
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
    use clean_base::entities::default_response::{
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
    use myc_http_tools::{middleware::ProfileData, utils::JsonError};
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;
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

    #[derive(Deserialize, IntoParams)]
    #[serde(rename_all = "camelCase")]
    pub struct CreateRoleParams {
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
        path = "/managers/roles/",
        params(
            CreateRoleParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
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
        info: web::Query<CreateRoleParams>,
        profile: ProfileData,
        role_registration_repo: Inject<
            RoleRegistrationModule,
            dyn RoleRegistration,
        >,
    ) -> impl Responder {
        match create_role(
            profile.to_profile(),
            info.name.to_owned(),
            info.description.to_owned(),
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
        path = "/myc/managers/roles/",
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
                status = 200,
                description = "Success.",
                body = [Role],
            ),
        ),
    )]
    #[get("/")]
    pub async fn list_roles_url(
        info: web::Query<ListRolesParams>,
        profile: ProfileData,
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
            },
        }
    }

    /// Delete Role
    ///
    /// Delete a single role.
    #[utoipa::path(
        delete,
        path = "/myc/managers/roles/{role}/delete",
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
                status = 204,
                description = "Role deleted.",
            ),
        ),
    )]
    #[delete("/{role}/delete")]
    pub async fn delete_role_url(
        path: web::Path<Uuid>,
        profile: ProfileData,
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
        path = "/myc/managers/roles/{role}/update-name-and-description",
        params(
            ("role" = Uuid, Path, description = "The role primary key."),
            CreateRoleParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
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
        info: web::Query<CreateRoleParams>,
        profile: ProfileData,
        role_fetching_repo: Inject<RoleFetchingModule, dyn RoleFetching>,
        role_updating_repo: Inject<RoleUpdatingModule, dyn RoleUpdating>,
    ) -> impl Responder {
        match update_role_name_and_description(
            profile.to_profile(),
            path.to_owned(),
            info.name.to_owned(),
            info.description.to_owned(),
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
