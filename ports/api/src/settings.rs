// ? ---------------------------------------------------------------------------
// ? Routes
// ? ---------------------------------------------------------------------------

/// The scope used to indicate admin routes
pub const ADMIN_API_SCOPE: &str = "_adm";

/// The scope used to indicate tools routes
pub const TOOLS_API_SCOPE: &str = "tools";

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
