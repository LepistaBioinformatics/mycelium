pub mod account_endpoints;

use clean_base::dtos::enums::{ChildrenEnum, ParentEnum};
use myc_core::domain::dtos::account::{Account, AccountType, AccountTypeEnum};
use myc_http_tools::utils::JsonError;
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        account_endpoints::upgrade_account_privileges_url,
        account_endpoints::downgrade_account_privileges_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            ChildrenEnum<String, String>,
            ParentEnum<String, String>,

            // Schema models.
            Account,
            AccountType,
            AccountTypeEnum,
            JsonError,
        ),
    ),
    tags(
        (
            name = "service",
            description = "Service management endpoints."
        )
    ),
)]
pub struct ApiDoc;
