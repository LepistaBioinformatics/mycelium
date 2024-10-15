mod guest_manager;
mod no_role;
mod shared;
mod subscription_manager;
mod system_manager;
mod tenant_manager;
mod tenant_owner;
mod user_manager;

use super::shared::UrlGroup;
pub(crate) use crate::endpoints::standard::shared::build_actor_context;

use actix_web::{get, web, HttpResponse, Responder};
use guest_manager::{
    guest_role_endpoints as guest_manager_guest_role_endpoints,
    role_endpoints as guest_manager_role_endpoints,
};
use myc_core::{
    domain::{
        actors::ActorName,
        dtos::{
            account::{Account, VerboseStatus},
            account_type::AccountTypeV2,
            email::Email,
            error_code::ErrorCode,
            guest::Permissions,
            guest::{GuestRole, GuestUser},
            profile::{LicensedResources, Profile},
            role::Role,
            tag::Tag,
            user::{PasswordHash, Provider, User},
            webhook::{AccountPropagationWebHookResponse, HookTarget, WebHook},
        },
    },
    use_cases::roles::standard::{
        guest_manager::guest_role::ActionType,
        no_role::user::EmailRegistrationStatus,
    },
};
use myc_http_tools::utils::HttpJsonResponse;
use mycelium_base::dtos::{Children, PaginatedRecord, Parent};
use no_role::{
    account_endpoints as no_role_account_endpoints,
    auxiliary_endpoints as no_role_auxiliary_endpoints,
    guest_endpoints as no_role_guest_endpoints,
    profile_endpoints as no_role_profile_endpoints,
    user_endpoints as no_role_user_endpoints,
};
use subscription_manager::{
    account_endpoints as subscription_manager_account_endpoints,
    guest_endpoints as subscription_manager_guest_endpoints,
    tag_endpoints as subscription_manager_tag_endpoints,
};
use system_manager::{
    error_code_endpoints as system_manager_error_code_endpoints,
    webhook_endpoints as system_manager_webhook_endpoints,
};
use tenant_manager::{
    account_endpoints as tenant_manager_account_endpoints,
    tag_endpoints as tenant_manager_tag_endpoints,
};
use tenant_owner::{
    account_endpoints as tenant_owner_account_endpoints,
    meta_endpoints as tenant_owner_meta_endpoints,
    owner_endpoints as tenant_owner_owner_endpoints,
    tenant_endpoints as tenant_owner_tenant_endpoints,
};
use user_manager::account_endpoints as user_manager_account_endpoints;
use utoipa::OpenApi;

#[get("/")]
pub async fn list_role_controlled_main_routes_url() -> impl Responder {
    HttpResponse::Ok().json(
        [
            ActorName::NoRole,
            ActorName::SubscriptionManager,
            ActorName::UserManager,
            ActorName::GuestManager,
            ActorName::SystemManager,
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
            .map(|group| build_actor_context(actor.to_owned(), group))
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
            web::scope(&format!("/{}", ActorName::NoRole.to_string().as_str()))
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
        // Guest Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                ActorName::GuestManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Roles))
                    .configure(guest_manager_role_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::GuestRoles))
                    .configure(guest_manager_guest_role_endpoints::configure),
            ),
        )
        //
        // Subscription Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                ActorName::SubscriptionManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts)).configure(
                    subscription_manager_account_endpoints::configure,
                ),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tags))
                    .configure(subscription_manager_tag_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Guests))
                    .configure(subscription_manager_guest_endpoints::configure),
            ),
        )
        //
        // System Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                ActorName::SystemManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::ErrorCodes))
                    .configure(system_manager_error_code_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Webhooks))
                    .configure(system_manager_webhook_endpoints::configure),
            ),
        )
        //
        // Tenant Manager
        //
        .service(
            web::scope(&format!(
                "/{}",
                ActorName::TenantManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(tenant_manager_account_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tags))
                    .configure(tenant_manager_tag_endpoints::configure),
            ),
        )
        //
        // Tenant Owner
        //
        .service(
            web::scope(&format!(
                "/{}",
                ActorName::TenantOwner.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(tenant_owner_account_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Meta))
                    .configure(tenant_owner_meta_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Owners))
                    .configure(tenant_owner_owner_endpoints::configure),
            )
            .service(
                web::scope(&format!("/{}", UrlGroup::Tenants))
                    .configure(tenant_owner_tenant_endpoints::configure),
            ),
        )
        //
        // User Accounts Managers
        //
        .service(
            web::scope(&format!(
                "/{}",
                ActorName::UserManager.to_string().as_str()
            ))
            .service(
                web::scope(&format!("/{}", UrlGroup::Accounts))
                    .configure(user_manager_account_endpoints::configure),
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
        no_role_user_endpoints::start_password_redefinition_url,
        no_role_user_endpoints::check_token_and_reset_password_url,
        no_role_user_endpoints::check_email_password_validity_url,
        subscription_manager_account_endpoints::create_subscription_account_url,
        subscription_manager_account_endpoints::update_account_name_and_flags_url,
        subscription_manager_account_endpoints::list_accounts_by_type_url,
        subscription_manager_account_endpoints::get_account_details_url,
        subscription_manager_account_endpoints::propagate_existing_subscription_account_url,
        subscription_manager_tag_endpoints::register_tag_url,
        subscription_manager_tag_endpoints::update_tag_url,
        subscription_manager_tag_endpoints::delete_tag_url,
        subscription_manager_guest_endpoints::list_licensed_accounts_of_email_url,
        subscription_manager_guest_endpoints::guest_user_url,
        subscription_manager_guest_endpoints::uninvite_guest_url,
        subscription_manager_guest_endpoints::update_user_guest_role_url,
        subscription_manager_guest_endpoints::list_guest_on_subscription_account_url,
        user_manager_account_endpoints::approve_account_url,
        user_manager_account_endpoints::disapprove_account_url,
        user_manager_account_endpoints::activate_account_url,
        user_manager_account_endpoints::deactivate_account_url,
        user_manager_account_endpoints::archive_account_url,
        user_manager_account_endpoints::unarchive_account_url,
        system_manager_error_code_endpoints::register_error_code_url,
        system_manager_error_code_endpoints::list_error_codes_url,
        system_manager_error_code_endpoints::get_error_code_url,
        system_manager_error_code_endpoints::update_error_code_message_and_details_url,
        system_manager_error_code_endpoints::delete_error_code_url,
        system_manager_webhook_endpoints::crate_webhook_url,
        system_manager_webhook_endpoints::delete_webhook_url,
        system_manager_webhook_endpoints::list_webhooks_url,
        system_manager_webhook_endpoints::update_webhook_url,
        guest_manager_guest_role_endpoints::crate_guest_role_url,
        guest_manager_guest_role_endpoints::list_guest_roles_url,
        guest_manager_guest_role_endpoints::delete_guest_role_url,
        guest_manager_guest_role_endpoints::update_guest_role_name_and_description_url,
        guest_manager_guest_role_endpoints::update_guest_role_permissions_url,
        guest_manager_role_endpoints::crate_role_url,
        guest_manager_role_endpoints::list_roles_url,
        guest_manager_role_endpoints::delete_role_url,
        guest_manager_role_endpoints::update_role_name_and_description_url,
        tenant_owner_account_endpoints::create_management_account_url,
        tenant_owner_meta_endpoints::create_tenant_meta_url,
        tenant_owner_meta_endpoints::delete_tenant_meta_url,
        tenant_owner_meta_endpoints::update_tenant_meta_url,
        tenant_owner_owner_endpoints::guest_tenant_owner_url,
        tenant_owner_owner_endpoints::revoke_tenant_owner_url,
        tenant_owner_tenant_endpoints::update_tenant_name_and_description_url,
        tenant_owner_tenant_endpoints::update_tenant_archiving_status_url,
        tenant_owner_tenant_endpoints::update_tenant_trashing_status_url,
        tenant_owner_tenant_endpoints::update_tenant_verifying_status_url,
        tenant_manager_account_endpoints::delete_subscription_account_url,
        tenant_manager_tag_endpoints::register_tag_url,
        tenant_manager_tag_endpoints::update_tag_url,
        tenant_manager_tag_endpoints::delete_tag_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            Children<String, String>,
            Parent<String, String>,

            // Schema models.
            Account,
            AccountTypeV2,
            ActionType,
            ActorName,
            HttpJsonResponse,
            LicensedResources,
            PasswordHash,
            Profile,
            Provider,
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
            Tag,
            system_manager_webhook_endpoints::UpdateWebHookBody,
            guest_manager_guest_role_endpoints::UpdateGuestRolePermissionsBody,
            no_role_account_endpoints::CreateDefaultAccountBody,
            no_role_account_endpoints::UpdateOwnAccountNameAccountBody,
            no_role_guest_endpoints::GuestUserBody,
            no_role_user_endpoints::CheckEmailStatusBody,
            no_role_user_endpoints::CreateDefaultUserBody,
            no_role_user_endpoints::CheckTokenBody,
            no_role_user_endpoints::StartPasswordResetBody,
            no_role_user_endpoints::ResetPasswordBody,
            no_role_user_endpoints::CheckUserCredentialsBody,
            tenant_owner_meta_endpoints::CreateTenantMetaBody,
            tenant_owner_meta_endpoints::DeleteTenantMetaBody,
            tenant_owner_owner_endpoints::GuestTenantOwnerBody,
            tenant_owner_tenant_endpoints::UpdateTenantNameAndDescriptionBody,
            tenant_owner_tenant_endpoints::UpdateTenantArchivingBody,
            tenant_owner_tenant_endpoints::UpdateTenantTrashingBody,
            tenant_owner_tenant_endpoints::UpdateTenantVerifyingBody,
            tenant_manager_tag_endpoints::CreateTagBody,
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
