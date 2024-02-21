pub mod account_endpoints;

use myc_core::domain::dtos::account::{Account, AccountType, AccountTypeEnum};
use myc_http_tools::utils::JsonError;
use mycelium_base::dtos::{Children, Parent};
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
            Children<String, String>,
            Parent<String, String>,

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
