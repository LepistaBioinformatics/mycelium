/// The assertion kind carrying a subject's profile grants (P8, FED-39)
///
/// Kinds are compared as text so a kind this instance does not know is
/// ignored, never an error (FED-41).
///
pub const PROFILE_ASSERTION_KIND: &str = "profile.v1";

/// The assertion kind carrying an instance's guest-role catalog, with no user
/// subject: `sub` is the issuer's own instance id (P8, FED-39)
pub const GUEST_ROLE_VOCABULARY_KIND: &str = "guest-role-vocabulary.v1";
