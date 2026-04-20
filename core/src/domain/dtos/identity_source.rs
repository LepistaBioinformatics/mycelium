use serde::{Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};

/// The source platform from which the request identity is extracted when a
/// route uses body-passthrough authentication.
///
/// When set on a `Route`, `check_security_group` resolves the caller's
/// identity from the request body instead of a JWT or connection string.
/// Source reliability (IP allowlist) is mandatory and enforced before
/// extraction.
///
/// Add a new messaging IdP by:
/// 1. Adding a variant here.
/// 2. Implementing `BodyIdpResolver` for it in `ports/api`.
/// 3. Registering it in `build_body_idp_resolver`.
#[derive(
    Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum IdentitySource {
    /// Identity resolved from `from.id` in the Telegram update JSON body.
    Telegram,
}
