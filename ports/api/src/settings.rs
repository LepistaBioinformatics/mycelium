// ? ---------------------------------------------------------------------------
// ? Routes
// ? ---------------------------------------------------------------------------

/// The scope used to indicate admin routes
pub const ADMIN_API_SCOPE: &str = "adm";

/// The scope used to indicate gateway routes
pub const GATEWAY_API_SCOPE: &str = "gw";

/// The scope used to indicate tools routes
pub const TOOLS_API_SCOPE: &str = "tools";

/// The scope used to indicate MCP routes
pub const MCP_API_SCOPE: &str = "mcp";

/// The scope used to indicate super user routes
pub const SUPER_USER_API_SCOPE: &str = "su";

/// The scope used to indicate role scoped routes
pub const ROLE_SCOPED_API_SCOPE: &str = "rs";

// ? ---------------------------------------------------------------------------
// ? Health check
// ? ---------------------------------------------------------------------------

/// The prefix used to indicate mycelium health check operation code
const MYC_HEALTH_CHECK_PREFIX: &str = "myc.hc";

/// Concatenate the health check prefix with the key
///
/// This function concatenates the health check prefix with the key and returns
/// the concatenated string.
///
pub(crate) fn build_health_check_key(key: &str) -> String {
    format!("{MYC_HEALTH_CHECK_PREFIX}.{key}").clone()
}

pub(crate) const MYC_OPERATION_CODE: &str = "myc.hc.operation_code";

pub(crate) const MYC_IS_HOST_HEALTHY: &str = "myc.hc.is_host_healthy";
