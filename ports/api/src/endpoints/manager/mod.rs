pub(crate) mod tenant_endpoints;

use myc_core::domain::dtos::{
    account::Account,
    account_type::AccountTypeV2,
    tag::Tag,
    tenant::{Tenant, TenantMetaKey, TenantStatus},
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
        tenant_endpoints::create_tenant_url,
        tenant_endpoints::list_tenant_url,
        tenant_endpoints::delete_tenant_url,
        tenant_endpoints::include_tenant_owner_url,
        tenant_endpoints::exclude_tenant_owner_url,
    ),
    components(
        schemas(
            // Default relationship enumerators.
            Children<String, String>,
            Parent<String, String>,

            // Schema models.
            Account,
            AccountTypeV2,
            Tag,
            Tenant,
            TenantMetaKey,
            TenantStatus,
            HttpJsonResponse,
            tenant_endpoints::CreateTenantBody,
        ),
    ),
    tags(
        (
            name = "manager",
            description = "Portal Manager endpoints"
        )
    ),
)]
pub(crate) struct ApiDoc;
