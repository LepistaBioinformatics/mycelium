mod guest_manager;
mod no_role;
mod shared;
mod subscription_account_manager;
mod system_manager;
mod user_account_manager;

use super::shared::UrlGroup;
use crate::endpoints::standard::shared::build_actor_context;

use guest_manager::{
    guest_endpoints as guest_manager_guest_endpoints,
    guest_role_endpoints as guest_manager_guest_role_endpoints,
    role_endpoints as guest_manager_role_endpoints,
};
use no_role::{
    account_endpoints as no_role_account_endpoints,
    auxiliary_endpoints as no_role_auxiliary_endpoints,
    guest_endpoints as no_role_guest_endpoints,
    profile_endpoints as no_role_profile_endpoints,
    user_endpoints as no_role_user_endpoints,
};
use subscription_account_manager::account_endpoints as subscription_account_manager_account_endpoints;
use system_manager::{
    error_code_endpoints as system_manager_error_code_endpoints,
    webhook_endpoints as system_manager_webhook_endpoints,
};
use user_account_manager::account_endpoints as user_account_manager_account_endpoints;

use actix_web::{get, web, HttpResponse, Responder};
use myc_core::{
    domain::{
        actors::DefaultActor,
        dtos::{
            account::{Account, AccountType, AccountTypeEnum, VerboseStatus},
            email::Email,
            error_code::ErrorCode,
            guest::Permissions,
            guest::{GuestRole, GuestUser},
            profile::{LicensedResources, Profile},
            role::Role,
            user::User,
            webhook::{AccountPropagationWebHookResponse, HookTarget, WebHook},
        },
    },
    use_cases::roles::standard::{
        guest_manager::guest_role::ActionType,
        no_role::user::EmailRegistrationStatus,
    },
};
use myc_http_tools::utils::JsonError;
use mycelium_base::dtos::{Children, PaginatedRecord, Parent};
use utoipa::OpenApi;

#[get("/")]
pub async fn list_role_controlled_main_routes_url() -> impl Responder {
    HttpResponse::Ok().json(
        [
            DefaultActor::NoRole,
            DefaultActor::SubscriptionAccountManager,
            DefaultActor::UserAccountManager,
            DefaultActor::GuestManager,
            DefaultActor::SystemManager,
        ]
        .into_iter()
        .flat_map(|actor| {
            [
                UrlGroup::Accounts,
                UrlGroup::GuestRoles,
                UrlGroup::Guests,
                UrlGroup::Roles,
                UrlGroup::Users,
                UrlGroup::Webhooks,
                UrlGroup::ErrorCodes,
                UrlGroup::Profile,
            ]
            .into_iter()
            .map(|group| build_actor_context(actor, group))
            .collect::<Vec<String>>()
        })
        .into_iter()
        .collect::<Vec<String>>(),
    )
}

// ? ---------------------------------------------------------------------------
// ? Configure application re-routing
// ? ---------------------------------------------------------------------------

pub(crate) fn configure(config: &mut web::ServiceConfig) {
    config
        .service(list_role_controlled_main_routes_url)
        .service(
            web::scope("/aux")
                .configure(no_role_auxiliary_endpoints::configure),
        )
        //
        // NoRole
        //
        .service(
            web::scope(&format!(
                "/{}",
                DefaultActor::NoRole.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(no_role_account_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Guests))
                    .configure(no_role_guest_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Profile))
                    .configure(no_role_profile_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Users))
                    .configure(no_role_user_endpoints::configure),
            ),
        )
        //
        // Subscription Accounts Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                DefaultActor::SubscriptionAccountManager
                    .to_string()
                    .as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts)).configure(
                    subscription_account_manager_account_endpoints::configure,
                ),
            ),
        )
        //
        // User Accounts Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                DefaultActor::UserAccountManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts)).configure(
                    user_account_manager_account_endpoints::configure,
                ),
            ),
        )
        //
        // Guest Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                DefaultActor::GuestManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Roles))
                    .configure(guest_manager_role_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Guests))
                    .configure(guest_manager_guest_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::GuestRoles))
                    .configure(guest_manager_guest_role_endpoints::configure),
            ),
        )
        //
        // System Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                DefaultActor::SystemManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::ErrorCodes))
                    .configure(system_manager_error_code_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Webhooks))
                    .configure(system_manager_webhook_endpoints::configure),
            ),
        );
}

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        no_role_auxiliary_endpoints::list_actors_url,
        no_role_account_endpoints::create_default_account_url,
        no_role_account_endpoints::update_own_account_name_url,
        no_role_guest_endpoints::guest_to_default_account_url,
        no_role_profile_endpoints::fetch_profile,
        no_role_user_endpoints::check_email_registration_status_url,
        no_role_user_endpoints::create_default_user_url,
        no_role_user_endpoints::check_user_token_url,
        no_role_user_endpoints::check_password_change_token_url,
        no_role_user_endpoints::check_email_password_validity_url,
        subscription_account_manager_account_endpoints::create_subscription_account_url,
        subscription_account_manager_account_endpoints::update_account_name_and_flags_url,
        subscription_account_manager_account_endpoints::list_accounts_by_type_url,
        subscription_account_manager_account_endpoints::get_account_details_url,
        subscription_account_manager_account_endpoints::propagate_existing_subscription_account_url,
        subscription_account_manager_account_endpoints::register_tag_url,
        subscription_account_manager_account_endpoints::update_tag_url,
        subscription_account_manager_account_endpoints::delete_tag_url,
        user_account_manager_account_endpoints::approve_account_url,
        user_account_manager_account_endpoints::disapprove_account_url,
        user_account_manager_account_endpoints::activate_account_url,
        user_account_manager_account_endpoints::deactivate_account_url,
        user_account_manager_account_endpoints::archive_account_url,
        user_account_manager_account_endpoints::unarchive_account_url,
        system_manager_error_code_endpoints::register_error_code_url,
        system_manager_error_code_endpoints::list_error_codes_url,
        system_manager_error_code_endpoints::get_error_code_url,
        system_manager_error_code_endpoints::update_error_code_message_and_details_url,
        system_manager_error_code_endpoints::delete_error_code_url,
        system_manager_webhook_endpoints::crate_webhook_url,
        system_manager_webhook_endpoints::delete_webhook_url,
        system_manager_webhook_endpoints::list_webhooks_url,
        system_manager_webhook_endpoints::update_webhook_url,
        guest_manager_guest_endpoints::list_licensed_accounts_of_email_url,
        guest_manager_guest_endpoints::guest_user_url,
        guest_manager_guest_endpoints::uninvite_guest_url,
        guest_manager_guest_endpoints::update_user_guest_role_url,
        guest_manager_guest_endpoints::list_guest_on_subscription_account_url,
        guest_manager_guest_role_endpoints::crate_guest_role_url,
        guest_manager_guest_role_endpoints::list_guest_roles_url,
        guest_manager_guest_role_endpoints::delete_guest_role_url,
        guest_manager_guest_role_endpoints::update_guest_role_name_and_description_url,
        guest_manager_guest_role_endpoints::update_guest_role_permissions_url,
        guest_manager_role_endpoints::crate_role_url,
        guest_manager_role_endpoints::list_roles_url,
        guest_manager_role_endpoints::delete_role_url,
        guest_manager_role_endpoints::update_role_name_and_description_url,
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
            JsonError,
            LicensedResources,
            Profile,
            Permissions,
            VerboseStatus,
            User,
            Email,
            EmailRegistrationStatus,
            ErrorCode,
            GuestUser,
            GuestRole,
            PaginatedRecord<Account>,
            PaginatedRecord<ErrorCode>,
            Role,
            HookTarget,
            WebHook,
            AccountPropagationWebHookResponse,
            subscription_account_manager_account_endpoints::CreateSubscriptionAccountBody,
            subscription_account_manager_account_endpoints::CreateTagBody,
            system_manager_webhook_endpoints::UpdateWebHookBody,
            guest_manager_guest_role_endpoints::UpdateGuestRolePermissionsBody,
        ),
    ),
    tags(
        (
            name = "standard-users",
            description = "Standard Users management endpoints."
        )
    ),
)]
pub struct ApiDoc;
