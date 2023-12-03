/// The scope used to redirect users to gateway
pub const GATEWAY_API_SCOPE: &str = "gw";

/// The scope used to redirect users to the mycelium functionalities
pub const MYCELIUM_API_SCOPE: &str = "myc";

/// When user is logged in, this key is used to store the user's id in the
/// session
pub(crate) const SESSION_KEY_USER_ID: &str = "myc-session-uid";

/// When user is logged in, this key is used to store the user's id in the
/// session
pub(crate) const SESSION_KEY_EMAIL: &str = "myc-session-email";
