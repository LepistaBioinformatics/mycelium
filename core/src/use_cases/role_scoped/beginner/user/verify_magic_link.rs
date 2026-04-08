use crate::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{PasswordHash, Provider, User},
    },
    entities::{TokenInvalidation, UserFetching, UserRegistration},
};

use chrono::Local;
use mycelium_base::{
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Verify a magic link code and issue a user session.
///
/// Steps:
/// 1. Consume the magic link code (phase 2 — record deleted).
/// 2. Fetch or auto-create the User for the email.
/// 3. Return the User so the handler can encode a JWT.
#[tracing::instrument(name = "verify_magic_link", skip_all)]
pub async fn verify_magic_link(
    email: Email,
    code: String,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
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
    // ? Fetch or auto-create the User for this email
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo
        .get_not_redacted_user_by_email(email.clone())
        .await?
    {
        FetchResponseKind::Found(u) => u,
        FetchResponseKind::NotFound(_) => {
            // Auto-create a minimal active user. The password hash is a
            // random sentinel — the user never needs a password in the
            // magic link flow.
            let new_user = User::new(
                None,
                email.email(),
                email.clone(),
                None,
                None,
                true,
                Local::now(),
                None,
                None,
                Some(Provider::Internal(PasswordHash::hash_user_password(
                    Uuid::new_v4().to_string().as_bytes(),
                ))),
            );

            match user_registration_repo.get_or_create(new_user).await? {
                GetOrCreateResponseKind::Created(u) => u,
                GetOrCreateResponseKind::NotCreated(u, _) => u,
            }
        }
    };

    Ok(user)
}
