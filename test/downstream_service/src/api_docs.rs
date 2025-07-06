use utoipa::OpenApi;

use super::endpoints::{self as TestEndpoints};

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Test Server for Mycelium API Gateway",
        description = include_str!("redoc-intro.md"),
        license(
            name = "Apache 2.0",
            identifier = "Apache-2.0",
        ),
    ),
    paths(
        TestEndpoints::health,
        TestEndpoints::public,
        TestEndpoints::protected,
        TestEndpoints::protected_by_role,
        TestEndpoints::protected_by_role_with_permission,
        TestEndpoints::protected_by_service_token_with_scope,
        TestEndpoints::account_created_webhook,
        TestEndpoints::account_updated_webhook,
        TestEndpoints::account_deleted_webhook,
        TestEndpoints::expects_headers,
        TestEndpoints::test_authorization_header,
        TestEndpoints::test_query_parameter_token,
    ),
    components(
        schemas(
            TestEndpoints::WebHookBody,
        ),
    ),
)]
pub(crate) struct ApiDoc;
