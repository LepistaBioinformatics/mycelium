pub(crate) mod guest_endpoints;

use myc_core::domain::{
    actors::ActorName,
    dtos::{
        account::{Account, VerboseStatus},
        account_type::AccountTypeV2,
        tag::Tag,
    },
};
use myc_http_tools::utils::HttpJsonResponse;
use mycelium_base::dtos::{Children, Parent};
use utoipa::OpenApi;

// ? ---------------------------------------------------------------------------
// ? Configure the API documentation
// ? ---------------------------------------------------------------------------

#[derive(OpenApi)]
#[openapi(
    paths(
        guest_endpoints::guest_to_default_account_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            Children<String, String>,
            Parent<String, String>,

            // Schema models.
            Account,
            ActorName,
            AccountTypeV2,
            HttpJsonResponse,
            Tag,
            VerboseStatus,
            guest_endpoints::GuestUserBody,
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
