pub mod account_endpoints;
pub mod error_code_endpoints;
pub mod guest_endpoints;
pub mod guest_role_endpoints;
pub mod role_endpoints;

use clean_base::dtos::{Children, PaginatedRecord, Parent};
use myc_core::{
    domain::dtos::{
        account::{Account, AccountType, AccountTypeEnum, VerboseStatus},
        email::Email,
        error_code::ErrorCode,
        guest::{GuestRole, GuestUser, Permissions},
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
            Permissions,
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
