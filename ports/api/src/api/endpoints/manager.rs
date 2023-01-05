use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::{
    domain::dtos::{
        account::{Account, AccountType},
        email::Email,
        guest::{GuestRole, GuestUser, PermissionsType},
        profile::{LicensedResources, Profile},
        role::Role,
    },
    use_cases::managers::guest_role::ActionType,
};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the Customer Partner API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        account_endpoints::create_subscription_account_url,
        guest_endpoints::guest_user_url,
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
        AccountRegistrationModule, AccountTypeRegistrationModule,
        UserRegistrationModule,
    };

    use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
    use clean_base::entities::default_response::GetOrCreateResponseKind;
    use myc_core::{
        domain::entities::{
            AccountRegistration, AccountTypeRegistration, UserRegistration,
        },
        use_cases::managers::account::create_subscription_account,
    };
    use myc_http_tools::extractor::extract_profile;
    use serde::Deserialize;
    use shaku_actix::Inject;
    use utoipa::IntoParams;

    // ? -----------------------------------------------------------------------
    // ? Configure application
    // ? -----------------------------------------------------------------------

    pub fn configure(config: &mut web::ServiceConfig) {
        config.service(web::scope("/managers").service(
            web::scope("/account").service(create_subscription_account_url),
        ));
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
}

pub mod guest_endpoints {

    use crate::modules::{
        AccountFetchingModule, GuestUserRegistrationModule,
        MessageSendingModule,
    };

    use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
    use clean_base::entities::default_response::GetOrCreateResponseKind;
    use myc_core::{
        domain::{
            dtos::email::Email,
            entities::{
                AccountFetching, GuestUserRegistration, MessageSending,
            },
        },
        use_cases::managers::guest::guest_user,
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
}

pub mod guest_role_endpoints {

    use crate::modules::{
        GuestRoleDeletionModule, GuestRoleFetchingModule,
        GuestRoleRegistrationModule, GuestRoleUpdatingModule,
    };

    use actix_web::{
        delete, patch, post, web, HttpRequest, HttpResponse, Responder,
    };
    use clean_base::entities::default_response::{
        DeletionResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
    };
    use myc_core::{
        domain::{
            dtos::guest::PermissionsType,
            entities::{
                GuestRoleDeletion, GuestRoleFetching, GuestRoleRegistration,
                GuestRoleUpdating,
            },
        },
        use_cases::managers::guest_role::{
            create_guest_role, delete_guest_role,
            update_guest_role_name_and_description,
            update_guest_role_permissions, ActionType,
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
            web::scope("/managers").service(
                web::scope("/guest-role")
                    .service(crate_guest_role_url)
                    .service(delete_guest_role_url)
                    .service(update_guest_role_name_and_description_url)
                    .service(update_guest_role_permissions_url),
            ),
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
        pub permissions: Option<Vec<PermissionsType>>,
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
        path = "/managers/guest-role/{role}/",
        params(
            CreateGuestRoleParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
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
        req: HttpRequest,
        role_registration_repo: Inject<
            GuestRoleRegistrationModule,
            dyn GuestRoleRegistration,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match create_guest_role(
            profile,
            info.name.to_owned(),
            info.description.to_owned(),
            path.to_owned(),
            info.permissions.to_owned(),
            Box::new(&*role_registration_repo),
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

    /// Delete Guest Role
    ///
    /// Delete a single guest role.
    #[utoipa::path(
        delete,
        path = "/managers/guest-role/{role}/delete",
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Guest Role not deleted.",
                body = String,
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
        req: HttpRequest,
        role_deletion_repo: Inject<
            GuestRoleDeletionModule,
            dyn GuestRoleDeletion,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match delete_guest_role(
            profile,
            path.to_owned(),
            Box::new(&*role_deletion_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                DeletionResponseKind::NotDeleted(_, msg) => {
                    HttpResponse::BadRequest().body(msg)
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
        path = "/managers/guest-role/{role}/update-name-and-description",
        params(
            UpdateGuestRoleNameAndDescriptionParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Guest Role not deleted.",
                body = String,
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
        req: HttpRequest,
        role_fetching_repo: Inject<
            GuestRoleFetchingModule,
            dyn GuestRoleFetching,
        >,
        role_updating_repo: Inject<
            GuestRoleUpdatingModule,
            dyn GuestRoleUpdating,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match update_guest_role_name_and_description(
            profile,
            info.name.to_owned(),
            info.description.to_owned(),
            path.to_owned(),
            Box::new(&*role_fetching_repo),
            Box::new(&*role_updating_repo),
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

    /// Change permissions of Guest Role
    ///
    /// Upgrade or Downgrade permissions of Guest Role.
    #[utoipa::path(
        patch,
        path = "/managers/guest-role/{role}/update-permissions",
        params(
            UpdateGuestRolePermissionsParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Guest Role not deleted.",
                body = String,
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
        req: HttpRequest,
        role_fetching_repo: Inject<
            GuestRoleFetchingModule,
            dyn GuestRoleFetching,
        >,
        role_updating_repo: Inject<
            GuestRoleUpdatingModule,
            dyn GuestRoleUpdating,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match update_guest_role_permissions(
            profile,
            path.to_owned(),
            info.permission.to_owned(),
            info.action_type.to_owned(),
            Box::new(&*role_fetching_repo),
            Box::new(&*role_updating_repo),
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

    use actix_web::{
        delete, patch, post, web, HttpRequest, HttpResponse, Responder,
    };
    use clean_base::entities::default_response::{
        DeletionResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
    };
    use myc_core::{
        domain::entities::{
            RoleDeletion, RoleFetching, RoleRegistration, RoleUpdating,
        },
        use_cases::managers::role::{
            create_role, delete_role, update_role_name_and_description,
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
            web::scope("/managers").service(
                web::scope("/role")
                    .service(crate_role_url)
                    .service(delete_role_url)
                    .service(update_role_name_and_description_url),
            ),
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

    /// Create Role
    ///
    /// Roles are used to build Guest Role elements.
    #[utoipa::path(
        post,
        path = "/managers/role/",
        params(
            CreateRoleParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
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
        req: HttpRequest,
        role_registration_repo: Inject<
            RoleRegistrationModule,
            dyn RoleRegistration,
        >,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match create_role(
            profile,
            info.name.to_owned(),
            info.description.to_owned(),
            Box::new(&*role_registration_repo),
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

    /// Delete Role
    ///
    /// Delete a single role.
    #[utoipa::path(
        delete,
        path = "/managers/role/{role}/delete",
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Role not deleted.",
                body = String,
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
        req: HttpRequest,
        role_deletion_repo: Inject<RoleDeletionModule, dyn RoleDeletion>,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match delete_role(
            profile,
            path.to_owned(),
            Box::new(&*role_deletion_repo),
        )
        .await
        {
            Err(err) => {
                HttpResponse::InternalServerError().body(err.to_string())
            }
            Ok(res) => match res {
                DeletionResponseKind::NotDeleted(_, msg) => {
                    HttpResponse::BadRequest().body(msg)
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
        path = "/managers/role/{role}/update-name-and-description",
        params(
            CreateRoleParams,
        ),
        responses(
            (
                status = 500,
                description = "Unknown internal server error.",
                body = String,
            ),
            (
                status = 400,
                description = "Guest Role not deleted.",
                body = String,
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
        req: HttpRequest,
        role_fetching_repo: Inject<RoleFetchingModule, dyn RoleFetching>,
        role_updating_repo: Inject<RoleUpdatingModule, dyn RoleUpdating>,
    ) -> impl Responder {
        let profile = match extract_profile(req).await {
            Err(err) => return err,
            Ok(res) => res,
        };

        match update_role_name_and_description(
            profile,
            path.to_owned(),
            info.name.to_owned(),
            info.description.to_owned(),
            Box::new(&*role_fetching_repo),
            Box::new(&*role_updating_repo),
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
