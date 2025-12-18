use crate::endpoints::{index, manager, role_scoped, service, staff};
use crate::modifiers::security::MyceliumSecurity;

use myc_core::domain::dtos::{
    account, account_type, email, error_code, guest_role, guest_user, profile,
    tag, token, tenant, user, webhook, route, service as service_dtos, 
    http_secret
};
use myc_http_tools::{utils::HttpJsonResponse, SystemActor};
use mycelium_base::dtos::{Children, Parent};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? DEFINE ENDPOINT GROUPS
// ? ---------------------------------------------------------------------------

use index::heath_check_endpoints as Index__Heath_Check;
use manager::guest_role_endpoints as Managers__Guest_Role;
use manager::tenant_endpoints as Managers__Tenants;
use manager::account_endpoints as Managers__Accounts;
use role_scoped::account_manager::guest_endpoints as Account_Manager__Guest;
use role_scoped::account_manager::guest_role_endpoints as Account_Manager__Guest_Role;
use role_scoped::gateway_manager::route_endpoints as Gateway_Manager__Route;
use role_scoped::gateway_manager::service_endpoints as Gateway_Manager__Service;
use role_scoped::gateway_manager::tools_endpoints as Gateway_Manager__Tools;
use role_scoped::beginners::account_endpoints as Beginners__Account;
use role_scoped::beginners::meta_endpoints as Beginners__Meta;
use role_scoped::beginners::profile_endpoints as Beginners__Profile;
use role_scoped::beginners::user_endpoints as Beginners__User;
use role_scoped::beginners::tenant_endpoints as Beginners__Tenant;
use role_scoped::beginners::token_endpoints as Beginners__Token;
use role_scoped::guest_manager::guest_role_endpoints as Guest_Manager__Guest_Role;
use role_scoped::subscriptions_manager::account_endpoints as Subscriptions_Manager__Account;
use role_scoped::subscriptions_manager::guest_endpoints as Subscriptions_Manager__Guest;
use role_scoped::subscriptions_manager::guest_role_endpoints as Subscriptions_Manager__Guest_Role;
use role_scoped::subscriptions_manager::tag_endpoints as Subscriptions_Manager__Tag;
use role_scoped::system_manager::error_code_endpoints as System_Manager__Error_Code;
use role_scoped::system_manager::webhook_endpoints as System_Manager__Webhook;
use role_scoped::tenant_manager::account_endpoints as Tenant_Manager__Account;
use role_scoped::tenant_manager::guest_endpoints as Tenant_Manager__Guest;
use role_scoped::tenant_manager::tag_endpoints as Tenant_Manager__Tag;
use role_scoped::tenant_manager::tenant_endpoints as Tenant_Manager__Tenant;
use role_scoped::tenant_owner::account_endpoints as Tenant_Owner__Account;
use role_scoped::tenant_owner::meta_endpoints as Tenant_Owner__Meta;
use role_scoped::tenant_owner::owner_endpoints as Tenant_Owner__Owner;
use role_scoped::tenant_owner::tenant_endpoints as Tenant_Owner__Tenant;
use role_scoped::users_manager::account_endpoints as Users_Manager__Account;
use service::tools_endpoints as Service__Tools;
use staff::account_endpoints as Staffs__Accounts;

/// Manager Endpoints for Account Management
/// 
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Manager | Account Endpoints",
        description = "Endpoints reserved for the application managers to manage accounts",
    ),
    paths(
        Managers__Accounts::create_system_account_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct ManagersAccountsApiDoc;

/// Manager Endpoints for Guest Roles Management
/// 
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Manager | Guest Role Endpoints",
        description = "Endpoints reserved for the application managers to manage guest roles",
    ),
    paths(
        Managers__Guest_Role::create_system_roles_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct ManagersGuestRoleApiDoc;

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
    ),
    security(("Bearer" = [], "ConnectionString" = []))
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
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct StaffsAccountsApiDoc;

/// Service Endpoints for Tools Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Service | Tools Endpoints",
        description = "Endpoints reserved for the application service to manage tools",
    ),
    paths(Service__Tools::list_discoverable_services_url)
)]
struct ServiceToolsApiDoc;

/// Account Manager Endpoints for Guests Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Account Manager | Guest Endpoints",
        description = "Endpoints reserved for the application account managers to manage guests",
    ),
    paths(Account_Manager__Guest::guest_to_children_account_url),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct AccountManagerGuestApiDoc;

/// Account Manager Endpoints for Guest Roles Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Account Manager | Guest Roles Endpoints",
        description = "Endpoints reserved for the application account managers to manage guest roles",
    ),
    paths(
        Account_Manager__Guest_Role::list_guest_roles_url,
        Account_Manager__Guest_Role::fetch_guest_role_details_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct AccountManagerGuestRoleApiDoc;

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
        Beginners__Account::get_my_account_details_url,
        Beginners__Account::delete_my_account_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct BeginnersAccountApiDoc;

/// Role Scoped Endpoints for Beginner Users for Meta Management
/// 
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | Meta Endpoints",
        description = "Endpoints reserved for the beginner users to manage their meta data",
    ),
    paths(
        Beginners__Meta::create_account_meta_url,
        Beginners__Meta::update_account_meta_url,
        Beginners__Meta::delete_account_meta_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct BeginnersMetaApiDoc;

/// Role Scoped Endpoints for Beginner Users for Profile Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | Profile Endpoints",
        description = "Endpoints reserved for the beginner users to manage their profiles",
    ),
    paths(Beginners__Profile::fetch_profile_url),
    security(("Bearer" = [], "ConnectionString" = []))
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
    ),
)]
struct BeginnersUserApiDoc;

/// Role Scoped Endpoints for Beginner Users for Tenant Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | Tenant Endpoints",
        description = "Endpoints reserved for the beginner users to manage their tenants",
    ),
    paths(Beginners__Tenant::fetch_tenant_public_info_url),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct BeginnersTenantApiDoc;

/// Role Scoped Endpoints for Beginner Users for Tokens Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Beginners | Token Endpoints",
        description = "Endpoints reserved for the beginner users to manage their tokens",
    ),
    paths(
        Beginners__Token::create_connection_string_url,
        Beginners__Token::list_my_connection_strings_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct BeginnersTokenApiDoc;

/// Role Scoped Endpoints for Gateway Manager for Routes Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Gateway Manager | Route Endpoints",
        description = "Endpoints reserved for the application gateway managers to manage routes",
    ),
    paths(
        Gateway_Manager__Route::list_routes_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct GatewayManagerRouteApiDoc;

/// Role Scoped Endpoints for Gateway Manager for Services Management
/// 
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Gateway Manager | Service Endpoints",
        description = "Endpoints reserved for the application gateway managers to manage services",
    ),
    paths(
        Gateway_Manager__Service::list_services_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct GatewayManagerServiceApiDoc;

/// Role Scoped Endpoints for Gateway Manager for Tools Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Gateway Manager | Tools Endpoints",
        description = "Endpoints reserved for the application gateway managers to manage tools",
    ),
    paths(
        Gateway_Manager__Tools::list_operations_url,
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct GatewayManagerToolsApiDoc;

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
    ),
    security(("Bearer" = [], "ConnectionString" = []))
)]
struct GuestManagerGuestRoleApiDoc;

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
        Subscriptions_Manager__Account::create_role_associated_account_url,
        Subscriptions_Manager__Account::update_account_name_and_flags_url,
        Subscriptions_Manager__Account::list_accounts_by_type_url,
        Subscriptions_Manager__Account::get_account_details_url,
        Subscriptions_Manager__Account::propagate_existing_subscription_account_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
        Subscriptions_Manager__Tag::register_account_tag_url,
        Subscriptions_Manager__Tag::update_account_tag_url,
        Subscriptions_Manager__Tag::delete_account_tag_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct SubscriptionsManagerGuestApiDoc;

/// Role Scoped Endpoints for Subscriptions Manager for Guest Role Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Subscriptions Manager | Guest Role Endpoints", 
        description = "Endpoints reserved for the application subscriptions managers to manage guest roles",
    ),
    paths(
        Subscriptions_Manager__Guest_Role::list_guest_roles_url,
        Subscriptions_Manager__Guest_Role::fetch_guest_role_details_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct SubscriptionsManagerGuestRoleApiDoc;

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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    paths(
        Tenant_Owner__Account::create_management_account_url,
        Tenant_Owner__Account::delete_tenant_manager_account_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
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
    paths(
        Tenant_Manager__Account::delete_subscription_account_url,
        Tenant_Manager__Account::create_subscription_manager_account_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct TenantManagerAccountApiDoc;

/// Role Scoped Endpoints for Tenant Manager for Guest Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Manager | Guest Endpoints",
        description = "Endpoints reserved for the application tenant managers to manage guests",
    ),
    paths(
        Tenant_Manager__Guest::guest_user_to_subscription_manager_account_url,
        Tenant_Manager__Guest::revoke_user_guest_to_subscription_manager_account_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct TenantManagerGuestApiDoc;

/// Role Scoped Endpoints for Tenant Manager for Tag Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Manager | Tag Endpoints",
        description = "Endpoints reserved for the application tenant managers to manage tags",
    ),
    paths(
        Tenant_Manager__Tag::register_tenant_tag_url,
        Tenant_Manager__Tag::update_tenant_tag_url,
        Tenant_Manager__Tag::delete_tenant_tag_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct TenantManagerTagApiDoc;

/// Role Scoped Endpoints for Tenant Manager for Tenant Management
///
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Tenant Manager | Tenant Endpoints",
        description = "Endpoints reserved for the application tenant managers to manage tenants",
    ),
    paths(
        Tenant_Manager__Tenant::get_tenant_details_url,
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct TenantManagerTenantApiDoc;

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
    ),
    security(("Bearer" = [], "ConnectionString" = [])),
)]
struct UsersManagerAccountApiDoc;

// ? ---------------------------------------------------------------------------
// ? MAIN ENDPOINT GROUP
// ? ---------------------------------------------------------------------------
#[derive(OpenApi)]
#[openapi(
    info(
        title = "Mycelium API",
        description = include_str!("redoc-intro.md"),
        license(
            name = "Apache 2.0",
            identifier = "Apache-2.0",
        ),
    ),
    modifiers(&MyceliumSecurity),
    nest(
        //
        // Super User endpoints
        //
        (path = "/_adm/staffs/accounts", api = StaffsAccountsApiDoc),
        (path = "/_adm/managers/accounts", api = ManagersAccountsApiDoc),
        (path = "/_adm/managers/tenants", api = ManagersTenantsApiDoc),
        (path = "/_adm/managers/guest-roles", api = ManagersGuestRoleApiDoc),
        //
        // Beginner endpoints
        //
        (path = "/_adm/beginners/accounts", api = BeginnersAccountApiDoc),
        (path = "/_adm/beginners/meta", api = BeginnersMetaApiDoc),
        (path = "/_adm/beginners/profile", api = BeginnersProfileApiDoc),
        (path = "/_adm/beginners/users", api = BeginnersUserApiDoc),
        (path = "/_adm/beginners/tenants", api = BeginnersTenantApiDoc),
        (path = "/_adm/beginners/tokens", api = BeginnersTokenApiDoc),
        // Account Manager endpoints
        //
        (path = "/_adm/accounts-manager/guests", api = AccountManagerGuestApiDoc),
        (path = "/_adm/accounts-manager/guest-roles", api = AccountManagerGuestRoleApiDoc),
        //
        // Gateway Manager endpoints
        //
        (path = "/_adm/gateway-manager/routes", api = GatewayManagerRouteApiDoc),
        (path = "/_adm/gateway-manager/services", api = GatewayManagerServiceApiDoc),
        (path = "/_adm/gateway-manager/tools", api = GatewayManagerToolsApiDoc),
        //
        // Guest Manager Endpoints
        //
        (path = "/_adm/guests-manager/guest-roles", api = GuestManagerGuestRoleApiDoc),
        //
        // Subscriptions Manager Endpoints
        //
        (path = "/_adm/subscriptions-manager/accounts", api = SubscriptionsManagerAccountApiDoc),
        (path = "/_adm/subscriptions-manager/tags", api = SubscriptionsManagerTagApiDoc),
        (path = "/_adm/subscriptions-manager/guests", api = SubscriptionsManagerGuestApiDoc),
        (path = "/_adm/subscriptions-manager/guest-roles", api = SubscriptionsManagerGuestRoleApiDoc),
        //
        // System Manager Endpoints
        //
        (path = "/_adm/system-manager/error-codes", api = SystemManagerErrorCodeApiDoc),
        (path = "/_adm/system-manager/webhooks", api = SystemManagerWebhookApiDoc),
        //
        // Tenant Owner Endpoints
        //
        (path = "/_adm/tenant-owner/accounts", api = TenantOwnerAccountApiDoc),
        (path = "/_adm/tenant-owner/meta", api = TenantOwnerMetaApiDoc),
        (path = "/_adm/tenant-owner/owners", api = TenantOwnerOwnerApiDoc),
        (path = "/_adm/tenant-owner/tenants", api = TenantOwnerTenantApiDoc),
        //
        // Tenant Manager Endpoints
        //
        (path = "/_adm/tenant-manager/accounts", api = TenantManagerAccountApiDoc),
        (path = "/_adm/tenant-manager/guests", api = TenantManagerGuestApiDoc),
        (path = "/_adm/tenant-manager/tags", api = TenantManagerTagApiDoc),
        (path = "/_adm/tenant-manager/tenants", api = TenantManagerTenantApiDoc),
        //
        // Users Manager Endpoints
        //
        (path = "/_adm/users-manager/accounts", api = UsersManagerAccountApiDoc),
        //
        // Service endpoints
        //
        (path = "/tools", api = ServiceToolsApiDoc),
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
            Parent<account::Account, String>,

            //
            // APPLICATION SCHEMAS
            //
            SystemActor,
            account::Account,
            account::VerboseStatus,
            account_type::AccountType,
            email::Email,
            error_code::ErrorCode,
            guest_role::GuestRole,
            guest_role::Permission,
            http_secret::HttpSecret, 
            profile::Owner,
            profile::LicensedResource,
            profile::Profile,
            service_dtos::Service, 
            route::Route, 
            tag::Tag,
            tenant::Tenant,
            tenant::TenantMetaKey,
            tenant::TenantStatus,
            token::PublicConnectionStringInfo,
            user::User,
            webhook::WebHook,
            webhook::WebHookTrigger,

            //
            // MANAGER
            //
            Managers__Accounts::CreateSystemSubscriptionAccountBody,
            Managers__Accounts::ApiSystemActor,
            Managers__Tenants::CreateTenantBody,
            Managers__Tenants::ListTenantParams,

            //
            // ACCOUNT MANAGER
            //
            Account_Manager__Guest::GuestUserToChildrenBody,

            //
            // BEGINNERS
            //
            Beginners__Account::CreateDefaultAccountBody,
            Beginners__Account::UpdateOwnAccountNameAccountBody,
            Beginners__Meta::CreateAccountMetaBody,
            Beginners__Meta::DeleteAccountMetaParams,
            Beginners__User::TotpUpdatingValidationBody,
            Beginners__User::CreateDefaultUserBody,
            Beginners__User::CheckTokenBody,
            Beginners__User::StartPasswordResetBody,
            Beginners__User::ResetPasswordBody,
            Beginners__User::CheckUserCredentialsBody,
            Beginners__Token::CreateTokenBody,

            //
            // GATEWAY MANAGER
            //
            Gateway_Manager__Route::ListRoutesByServiceParams,
            Gateway_Manager__Service::ListServicesParams,

            //
            // GUEST MANAGER
            //
            Guest_Manager__Guest_Role::CreateGuestRoleBody,
            Guest_Manager__Guest_Role::UpdateGuestRoleNameAndDescriptionBody,
            Guest_Manager__Guest_Role::UpdateGuestRolePermissionsBody,
            Guest_Manager__Guest_Role::ListGuestRolesParams,

            //
            // SUBSCRIPTIONS MANAGER
            //
            Subscriptions_Manager__Account::CreateSubscriptionAccountBody,
            Subscriptions_Manager__Account::CreateRoleAssociatedAccountBody,
            Subscriptions_Manager__Account::UpdateSubscriptionAccountNameAndFlagsBody,
            Subscriptions_Manager__Account::APIAccountType,
            Subscriptions_Manager__Account::ListSubscriptionAccountParams,
            Subscriptions_Manager__Guest::GuestUserBody,
            Subscriptions_Manager__Guest::ListLicensedAccountsOfEmailParams,
            Subscriptions_Manager__Tag::CreateAccountTagBody,
            Subscriptions_Manager__Tag::UpdateAccountTagBody,
            Subscriptions_Manager__Tag::DeleteAccountTagParams,

            //
            // SYSTEM MANAGER
            //
            System_Manager__Error_Code::CreateErrorCodeBody,
            System_Manager__Error_Code::ListErrorCodesParams,
            System_Manager__Error_Code::UpdateErrorCodeMessageAndDetailsBody,
            System_Manager__Webhook::CreateWebHookBody,
            System_Manager__Webhook::UpdateWebHookBody,
            System_Manager__Webhook::ListWebHooksParams,

            //
            // TENANT MANAGER
            //
            Tenant_Manager__Tag::CreateTagBody,
            Tenant_Manager__Guest::GuestUserToSubscriptionManagerAccountBody,
            Tenant_Manager__Guest::RevokeUserGuestToSubscriptionManagerAccountParams,

            //
            // TENANT OWNER
            //
            Tenant_Owner__Meta::CreateTenantMetaBody,
            Tenant_Owner__Meta::DeleteTenantMetaBody,
            Tenant_Owner__Owner::GuestTenantOwnerBody,
            Tenant_Owner__Tenant::UpdateTenantNameAndDescriptionBody,
        ),
        responses(
            //
            // APPLICATION SCHEMAS
            //
            HttpJsonResponse,

            //
            // BEGINNERS
            //
            Beginners__User::MyceliumLoginResponse,
            Beginners__User::TotpActivationStartedResponse,
            Beginners__User::TotpActivationFinishedResponse,
            Beginners__User::CheckEmailStatusResponse,
            Beginners__Token::CreateTokenResponse,

            //
            // SERVICE
            //
            service::tools_endpoints::ListServicesResponse,
        ),
    ),
)]
pub(crate) struct ApiDoc;
