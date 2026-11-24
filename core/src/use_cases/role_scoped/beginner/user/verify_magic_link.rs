use super::ensure_principal_user_by_email;
use crate::domain::{
    dtos::{email::Email, native_error_codes::NativeErrorCodes, user::User},
    entities::{
        TokenInvalidation, UserFetching, UserRegistration, UserUpdating,
    },
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Verify a magic link code and issue a user session.
///
/// Steps:
/// 1. Consume the magic link code (phase 2 — record deleted).
/// 2. Fetch or auto-create the User for the email.
/// 3. If the user exists but is not principal, upgrade them (proven ownership).
/// 4. Return the User so the handler can encode a JWT.
#[tracing::instrument(name = "verify_magic_link", skip_all)]
pub async fn verify_magic_link(
    email: Email,
    code: String,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
    user_updating_repo: Box<&dyn UserUpdating>,
    token_invalidation_repo: Box<&dyn TokenInvalidation>,
) -> Result<User, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Consume the code (phase 2 — deletes record)
    // ? -----------------------------------------------------------------------

    match token_invalidation_repo
        .get_and_invalidate_magic_link_code(&email, &code)
        .await?
    {
        FetchResponseKind::NotFound(_) => {
            return use_case_err("Invalid or expired code")
                .with_code(NativeErrorCodes::MYC00008)
                .with_exp_true()
                .as_error()
        }
        FetchResponseKind::Found(()) => {}
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch or auto-create the principal User for this email
    // ? -----------------------------------------------------------------------

    ensure_principal_user_by_email(
        email,
        user_fetching_repo,
        user_registration_repo,
        user_updating_repo,
    )
    .await
}
