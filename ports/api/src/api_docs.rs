use crate::endpoints::{
    index, manager, service, staff, standard as role_scoped,
};

use myc_core::domain::dtos::{
    account, account_type, email, error_code, guest_role, guest_user, profile,
    role, tag, tenant, user, webhook,
};
use myc_http_tools::{utils::HttpJsonResponse, ActorName};
use mycelium_base::dtos::{Children, Parent};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? DEFINE ENDPOINT GROUPS
// ? ---------------------------------------------------------------------------

use index::heath_check_endpoints as Index__Heath_Check;
use manager::tenant_endpoints as Managers__Tenants;
use role_scoped::account_manager::guest_endpoints as Account_Manager__Guest;
use role_scoped::beginners::account_endpoints as Beginners__Account;
use role_scoped::beginners::profile_endpoints as Beginners__Profile;
use role_scoped::beginners::user_endpoints as Beginners__User;
use role_scoped::guest_manager::guest_role_endpoints as Guest_Manager__Guest_Role;
use role_scoped::guest_manager::role_endpoints as Guest_Manager__Role;
use role_scoped::guest_manager::token_endpoints as Guest_Manager__Token;
use role_scoped::subscriptions_manager::account_endpoints as Subscriptions_Manager__Account;
use role_scoped::subscriptions_manager::guest_endpoints as Subscriptions_Manager__Guest;
use role_scoped::subscriptions_manager::tag_endpoints as Subscriptions_Manager__Tag;
use role_scoped::system_manager::error_code_endpoints as System_Manager__Error_Code;
use role_scoped::system_manager::webhook_endpoints as System_Manager__Webhook;
use role_scoped::tenant_manager::account_endpoints as Tenant_Manager__Account;
use role_scoped::tenant_manager::tag_endpoints as Tenant_Manager__Tag;
use role_scoped::tenant_manager::token_endpoints as Tenant_Manager__Token;
use role_scoped::tenant_owner::account_endpoints as Tenant_Owner__Account;
use role_scoped::tenant_owner::meta_endpoints as Tenant_Owner__Meta;
use role_scoped::tenant_owner::owner_endpoints as Tenant_Owner__Owner;
use role_scoped::tenant_owner::tenant_endpoints as Tenant_Owner__Tenant;
use role_scoped::users_manager::account_endpoints as Users_Manager__Account;
use service::account_endpoints as Service__Account;
use service::auxiliary_endpoints as Service__Auxiliary;
use service::guest_endpoints as Service__Guest;
use staff::account_endpoints as Staffs__Accounts;

/// Manager Endpoints for Tenant Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Manager | Tenant Endpoints",
        description = "Endpoints reserved for the application managers to manage tenants",
    ),
    paths(
        Managers__Tenants::create_tenant_url,
        Managers__Tenants::list_tenant_url,
        Managers__Tenants::delete_tenant_url,
        Managers__Tenants::include_tenant_owner_url,
        Managers__Tenants::exclude_tenant_owner_url,
    )
)]
struct ManagersTenantsApiDoc;

/// Staff Endpoints for Accounts Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Staff | Accounts Endpoints",
        description = "Endpoints reserved for the application staffs to manage accounts",
    ),
    paths(
        Staffs__Accounts::upgrade_account_privileges_url,
        Staffs__Accounts::downgrade_account_privileges_url,
    )
)]
struct StaffsAccountsApiDoc;

/// Service Endpoints for Accounts Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Service | Account Endpoints",
        description = "Endpoints reserved for the application service to manage accounts",
    ),
    paths(Service__Account::create_subscription_account_url)
)]
struct ServiceAccountApiDoc;

/// Service Endpoints for Guests Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Service | Guest Endpoints",
        description = "Endpoints reserved for the application service to manage guests",
    ),
    paths(Service__Guest::guest_to_default_account_url)
)]
struct ServiceGuestApiDoc;

/// Service Endpoints for Auxiliary
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Service | Auxiliary Endpoints",
        description = "Endpoints reserved for the application service to view developers' auxiliary data",
    ),
    paths(
        Service__Auxiliary::list_actors_url,
        Service__Auxiliary::list_role_controlled_main_routes_url
    )
)]
struct ServiceAuxiliaryApiDoc;

/// Account Manager Endpoints for Guests Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Account Manager | Guest Endpoints",
        description = "Endpoints reserved for the application account managers to manage guests",
    ),
    paths(Account_Manager__Guest::guest_to_children_account_url)
)]
struct AccountManagerGuestApiDoc;

/// Role Scoped Endpoints for Beginner Users for Account Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | Account Endpoints",
        description = "Endpoints reserved for the beginner users to manage their accounts",
    ),
    paths(
        Beginners__Account::create_default_account_url,
        Beginners__Account::update_own_account_name_url,
    )
)]
struct BeginnersAccountApiDoc;

/// Role Scoped Endpoints for Beginner Users for Profile Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | Profile Endpoints",
        description = "Endpoints reserved for the beginner users to manage their profiles",
    ),
    paths(Beginners__Profile::fetch_profile_url)
)]
struct BeginnersProfileApiDoc;

/// Role Scoped Endpoints for Beginner Users for Users Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | User Endpoints",
        description = "Endpoints reserved for the beginner users to manage their users",
    ),
    paths(
        Beginners__User::check_email_registration_status_url,
        Beginners__User::create_default_user_url,
        Beginners__User::check_user_token_url,
        Beginners__User::start_password_redefinition_url,
        Beginners__User::check_token_and_reset_password_url,
        Beginners__User::check_email_password_validity_url,
        Beginners__User::totp_start_activation_url,
        Beginners__User::totp_finish_activation_url,
        Beginners__User::totp_check_token_url,
        Beginners__User::totp_disable_url,
    )
)]
struct BeginnersUserApiDoc;

/// Role Scoped Endpoints for Guest Manager for Guest Roles Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Guest Manager | Guest Role Endpoints",
        description = "Endpoints reserved for the application guest managers to manage guest roles",
    ),
    paths(
        Guest_Manager__Guest_Role::crate_guest_role_url,
        Guest_Manager__Guest_Role::list_guest_roles_url,
        Guest_Manager__Guest_Role::delete_guest_role_url,
        Guest_Manager__Guest_Role::update_guest_role_name_and_description_url,
        Guest_Manager__Guest_Role::update_guest_role_permissions_url,
        Guest_Manager__Guest_Role::insert_role_child_url,
        Guest_Manager__Guest_Role::remove_role_child_url,
    )
)]
struct GuestManagerGuestRoleApiDoc;

/// Role Scoped Endpoints for Guest Manager for Roles Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Guest Manager | Role Endpoints",
        description = "Endpoints reserved for the application guest managers to manage roles",
    ),
    paths(
        Guest_Manager__Role::crate_role_url,
        Guest_Manager__Role::list_roles_url,
        Guest_Manager__Role::delete_role_url,
        Guest_Manager__Role::update_role_name_and_description_url,
    )
)]
struct GuestManagerRoleApiDoc;

/// Role Scoped Endpoints for Guest Manager for Tokens Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Guest Manager | Token Endpoints",
        description = "Endpoints reserved for the application guest managers to manage tokens",
    ),
    paths(
        Guest_Manager__Token::create_default_account_associated_connection_string_url,
        Guest_Manager__Token::create_role_associated_connection_string_url,
    )
)]
struct GuestManagerTokenApiDoc;

/// Role Scoped Endpoints for Subscriptions Manager for Account Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Subscriptions Manager | Account Endpoints",
        description = "Endpoints reserved for the application subscriptions managers to manage accounts",
    ),
    paths(
        Subscriptions_Manager__Account::create_subscription_account_url,
        Subscriptions_Manager__Account::update_account_name_and_flags_url,
        Subscriptions_Manager__Account::list_accounts_by_type_url,
        Subscriptions_Manager__Account::get_account_details_url,
        Subscriptions_Manager__Account::propagate_existing_subscription_account_url,
    )
)]
struct SubscriptionsManagerAccountApiDoc;

/// Role Scoped Endpoints for Subscriptions Manager for Tag Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Subscriptions Manager | Tag Endpoints",
        description = "Endpoints reserved for the application subscriptions managers to manage tags",
    ),
    paths(
        Subscriptions_Manager__Tag::register_tag_url,
        Subscriptions_Manager__Tag::update_tag_url,
        Subscriptions_Manager__Tag::delete_tag_url,
    )
)]
struct SubscriptionsManagerTagApiDoc;

/// Role Scoped Endpoints for Subscriptions Manager for Guest Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Subscriptions Manager | Guest Endpoints",
        description = "Endpoints reserved for the application subscriptions managers to manage guests",
    ),
    paths(
        Subscriptions_Manager__Guest::list_licensed_accounts_of_email_url,
        Subscriptions_Manager__Guest::guest_user_url,
        Subscriptions_Manager__Guest::uninvite_guest_url,
        Subscriptions_Manager__Guest::list_guest_on_subscription_account_url,
    )
)]
struct SubscriptionsManagerGuestApiDoc;

/// Role Scoped Endpoints for System Manager for Error Code Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "System Manager | Error Code Endpoints",
        description = "Endpoints reserved for the application system managers to manage error codes",
    ),
    paths(
        System_Manager__Error_Code::register_error_code_url,
        System_Manager__Error_Code::list_error_codes_url,
        System_Manager__Error_Code::get_error_code_url,
        System_Manager__Error_Code::update_error_code_message_and_details_url,
        System_Manager__Error_Code::delete_error_code_url,
    )
)]
struct SystemManagerErrorCodeApiDoc;

/// Role Scoped Endpoints for System Manager for Webhook Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "System Manager | Webhook Endpoints",
        description = "Endpoints reserved for the application system managers to manage webhooks",
    ),
    paths(
        System_Manager__Webhook::crate_webhook_url,
        System_Manager__Webhook::delete_webhook_url,
        System_Manager__Webhook::list_webhooks_url,
        System_Manager__Webhook::update_webhook_url,
    )
)]
struct SystemManagerWebhookApiDoc;

/// Role Scoped Endpoints for Tenant Owner for Account Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Owner | Account Endpoints",
        description = "Endpoints reserved for the application tenant owners to manage accounts",
    ),
    paths(Tenant_Owner__Account::create_management_account_url,)
)]
struct TenantOwnerAccountApiDoc;

/// Role Scoped Endpoints for Tenant Owner for Meta Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Owner | Meta Endpoints",
        description = "Endpoints reserved for the application tenant owners to manage meta data",
    ),
    paths(
        Tenant_Owner__Meta::create_tenant_meta_url,
        Tenant_Owner__Meta::delete_tenant_meta_url,
        Tenant_Owner__Meta::update_tenant_meta_url,
    )
)]
struct TenantOwnerMetaApiDoc;

/// Role Scoped Endpoints for Tenant Owner for Owner Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Owner | Owner Endpoints",
        description = "Endpoints reserved for the application tenant owners to manage owners",
    ),
    paths(
        Tenant_Owner__Owner::guest_tenant_owner_url,
        Tenant_Owner__Owner::revoke_tenant_owner_url,
    )
)]
struct TenantOwnerOwnerApiDoc;

/// Role Scoped Endpoints for Tenant Owner for Tenant Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Owner | Tenant Endpoints",
        description = "Endpoints reserved for the application tenant owners to manage tenants",
    ),
    paths(
        Tenant_Owner__Tenant::update_tenant_name_and_description_url,
        Tenant_Owner__Tenant::update_tenant_archiving_status_url,
        Tenant_Owner__Tenant::update_tenant_trashing_status_url,
        Tenant_Owner__Tenant::update_tenant_verifying_status_url,
    )
)]
struct TenantOwnerTenantApiDoc;

/// Role Scoped Endpoints for Tenant Manager for Account Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Manager | Account Endpoints",
        description = "Endpoints reserved for the application tenant managers to manage accounts",
    ),
    paths(Tenant_Manager__Account::delete_subscription_account_url,)
)]
struct TenantManagerAccountApiDoc;

/// Role Scoped Endpoints for Tenant Manager for Tag Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Manager | Tag Endpoints",
        description = "Endpoints reserved for the application tenant managers to manage tags",
    ),
    paths(
        Tenant_Manager__Tag::register_tag_url,
        Tenant_Manager__Tag::update_tag_url,
        Tenant_Manager__Tag::delete_tag_url,
    )
)]
struct TenantManagerTagApiDoc;

/// Role Scoped Endpoints for Tenant Manager for Token Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Manager | Token Endpoints",
        description = "Endpoints reserved for the application tenant managers to manage tokens",
    ),
    paths(
        Tenant_Manager__Token::create_tenant_associated_connection_string_url,
    )
)]
struct TenantManagerTokenApiDoc;

/// Role Scoped Endpoints for Users Manager for Account Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Users Manager | Account Endpoints",
        description = "Endpoints reserved for the application users managers to manage accounts",
    ),
    paths(
        Users_Manager__Account::approve_account_url,
        Users_Manager__Account::disapprove_account_url,
        Users_Manager__Account::activate_account_url,
        Users_Manager__Account::deactivate_account_url,
        Users_Manager__Account::archive_account_url,
        Users_Manager__Account::unarchive_account_url,
    )
)]
struct UsersManagerAccountApiDoc;

// ? ---------------------------------------------------------------------------
// ? MAIN ENDPOINT GROUP
// ? ---------------------------------------------------------------------------
#[derive(OpenApi)]
#[openapi(
    nest(
        //
        // Super User endpoints
        //
        (path = "/adm/su/staffs/accounts", api = StaffsAccountsApiDoc),
        (path = "/adm/su/managers/tenants", api = ManagersTenantsApiDoc),
        //
        // Service endpoints
        //
        (path = "/adm/svc/accounts", api = ServiceAccountApiDoc),
        (path = "/adm/svc/guests", api = ServiceGuestApiDoc),
        (path = "/adm/svc/auxiliary", api = ServiceAuxiliaryApiDoc),
        //
        // Beginner endpoints
        //
        (path = "/adm/rs/begin/accounts", api = BeginnersAccountApiDoc),
        (path = "/adm/rs/begin/profile", api = BeginnersProfileApiDoc),
        (path = "/adm/rs/begin/users", api = BeginnersUserApiDoc),
        //
        // Account Manager endpoints
        //
        (path = "/adm/rs/accounts-manager/guests", api = AccountManagerGuestApiDoc),
        //
        // Guest Manager Endpoints
        //
        (path = "/adm/rs/guests-manager/guest-roles", api = GuestManagerGuestRoleApiDoc),
        (path = "/adm/rs/guests-manager/roles", api = GuestManagerRoleApiDoc),
        (path = "/adm/rs/guests-manager/tokens", api = GuestManagerTokenApiDoc),
        //
        // Subscriptions Manager Endpoints
        //
        (path = "/adm/rs/subscriptions-manager/accounts", api = SubscriptionsManagerAccountApiDoc),
        (path = "/adm/rs/subscriptions-manager/tags", api = SubscriptionsManagerTagApiDoc),
        (path = "/adm/rs/subscriptions-manager/guests", api = SubscriptionsManagerGuestApiDoc),
        //
        // System Manager Endpoints
        //
        (path = "/adm/rs/system-manager/error-codes", api = SystemManagerErrorCodeApiDoc),
        (path = "/adm/rs/system-manager/webhooks", api = SystemManagerWebhookApiDoc),
        //
        // Tenant Owner Endpoints
        //
        (path = "/adm/rs/tenant-owner/accounts", api = TenantOwnerAccountApiDoc),
        (path = "/adm/rs/tenant-owner/meta", api = TenantOwnerMetaApiDoc),
        (path = "/adm/rs/tenant-owner/owners", api = TenantOwnerOwnerApiDoc),
        (path = "/adm/rs/tenant-owner/tenants", api = TenantOwnerTenantApiDoc),
        //
        // Tenant Manager Endpoints
        //
        (path = "/adm/rs/tenant-manager/accounts", api = TenantManagerAccountApiDoc),
        (path = "/adm/rs/tenant-manager/tags", api = TenantManagerTagApiDoc),
        (path = "/adm/rs/tenant-manager/tokens", api = TenantManagerTokenApiDoc),
        //
        // Users Manager Endpoints
        //
        (path = "/adm/rs/users-manager/accounts", api = UsersManagerAccountApiDoc),
    ),
    paths(
        //
        // HEALTH CHECK
        //
        Index__Heath_Check::health_url,
        Index__Heath_Check::now_url,
    ),
    components(
        schemas(
            //
            // GENERIC SCHEMAS
            //
            Children<user::User, String>,
            Children<guest_user::GuestUser, String>,
            Children<profile::Owner, String>,
            Parent<role::Role, String>,
            Parent<account::Account, String>,

            //
            // APPLICATION SCHEMAS
            //
            ActorName,
            account::Account,
            account::VerboseStatus,
            account_type::AccountTypeV2,
            email::Email,
            error_code::ErrorCode,
            guest_role::GuestRole,
            guest_role::Permission,
            profile::Owner,
            profile::LicensedResource,
            profile::Profile,
            role::Role,
            tag::Tag,
            tenant::Tenant,
            tenant::TenantMetaKey,
            tenant::TenantStatus,
            user::User,
            webhook::WebHook,
            webhook::WebHookTrigger,

            //
            // MANAGER
            //
            manager::tenant_endpoints::CreateTenantBody,
            manager::tenant_endpoints::ListTenantParams,

            //
            // ACCOUNT MANAGER
            //
            role_scoped::account_manager::guest_endpoints::GuestUserBody,

            //
            // BEGINNERS
            //
            role_scoped::beginners::account_endpoints::CreateDefaultAccountBody,
            role_scoped::beginners::account_endpoints::UpdateOwnAccountNameAccountBody,
            role_scoped::beginners::user_endpoints::CheckEmailStatusBody,
            role_scoped::beginners::user_endpoints::TotpUpdatingValidationBody,
            role_scoped::beginners::user_endpoints::CreateDefaultUserBody,
            role_scoped::beginners::user_endpoints::CheckTokenBody,
            role_scoped::beginners::user_endpoints::StartPasswordResetBody,
            role_scoped::beginners::user_endpoints::ResetPasswordBody,
            role_scoped::beginners::user_endpoints::CheckUserCredentialsBody,

            //
            // GUEST MANAGER
            //
            role_scoped::guest_manager::guest_role_endpoints::CreateGuestRoleBody,
            role_scoped::guest_manager::guest_role_endpoints::UpdateGuestRoleNameAndDescriptionBody,
            role_scoped::guest_manager::guest_role_endpoints::UpdateGuestRolePermissionsBody,
            role_scoped::guest_manager::guest_role_endpoints::ListGuestRolesParams,
            role_scoped::guest_manager::role_endpoints::CreateRoleBody,
            role_scoped::guest_manager::role_endpoints::ListRolesParams,
            role_scoped::guest_manager::token_endpoints::CreateTokenBody,

            //
            // SUBSCRIPTIONS MANAGER
            //
            role_scoped::subscriptions_manager::account_endpoints::CreateSubscriptionAccountBody,
            role_scoped::subscriptions_manager::account_endpoints::UpdateSubscriptionAccountNameAndFlagsBody,
            role_scoped::subscriptions_manager::account_endpoints::APIAccountType,
            role_scoped::subscriptions_manager::account_endpoints::ListSubscriptionAccountParams,
            role_scoped::subscriptions_manager::guest_endpoints::GuestUserBody,
            role_scoped::subscriptions_manager::guest_endpoints::ListLicensedAccountsOfEmailParams,
            role_scoped::subscriptions_manager::tag_endpoints::CreateTagBody,
            role_scoped::subscriptions_manager::tag_endpoints::UpdateTagBody,

            //
            // SYSTEM MANAGER
            //
            role_scoped::system_manager::error_code_endpoints::CreateErrorCodeBody,
            role_scoped::system_manager::error_code_endpoints::ListErrorCodesParams,
            role_scoped::system_manager::error_code_endpoints::UpdateErrorCodeMessageAndDetailsBody,
            role_scoped::system_manager::webhook_endpoints::CreateWebHookBody,
            role_scoped::system_manager::webhook_endpoints::UpdateWebHookBody,
            role_scoped::system_manager::webhook_endpoints::ListWebHooksParams,

            //
            // TENANT MANAGER
            //
            role_scoped::tenant_manager::tag_endpoints::CreateTagBody,
            role_scoped::tenant_manager::token_endpoints::CreateTokenBody,

            //
            // TENANT OWNER
            //
            role_scoped::tenant_owner::meta_endpoints::CreateTenantMetaBody,
            role_scoped::tenant_owner::meta_endpoints::DeleteTenantMetaBody,
            role_scoped::tenant_owner::owner_endpoints::GuestTenantOwnerBody,
            role_scoped::tenant_owner::tenant_endpoints::UpdateTenantNameAndDescriptionBody,
            role_scoped::tenant_owner::tenant_endpoints::UpdateTenantArchivingBody,
            role_scoped::tenant_owner::tenant_endpoints::UpdateTenantTrashingBody,
            role_scoped::tenant_owner::tenant_endpoints::UpdateTenantVerifyingBody,
        ),
        responses(
            //
            // APPLICATION SCHEMAS
            //
            HttpJsonResponse,

            //
            // BEGINNERS
            //
            role_scoped::beginners::user_endpoints::LoginResponse,
            role_scoped::beginners::user_endpoints::TotpActivationStartedResponse,
            role_scoped::beginners::user_endpoints::TotpActivationFinishedResponse,

            //
            // GUEST MANAGER
            //
            role_scoped::guest_manager::token_endpoints::CreateTokenResponse,

            //
            // TENANT MANAGER
            //
            role_scoped::tenant_manager::token_endpoints::CreateTokenResponse,
        ),
    ),
    tags(
        (
            name = "Mycelium API", 
            description = "The Mycelium API documentation"
        )
    ),
)]
pub(crate) struct ApiDoc;
