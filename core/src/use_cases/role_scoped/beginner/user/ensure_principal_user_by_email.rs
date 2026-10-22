use crate::domain::{
    dtos::{
        email::Email,
        user::{PasswordHash, Provider, User},
    },
    entities::{UserFetching, UserRegistration, UserUpdating},
};

use chrono::Local;
use mycelium_base::{
    entities::{
        FetchResponseKind, GetOrCreateResponseKind, UpdatingResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

/// Return the principal User for an email whose ownership was just proven
/// (magic link or hosted login code).
///
/// An existing user is upgraded to principal so it can create its own
/// account. A missing user is created as a minimal active principal user.
#[tracing::instrument(name = "ensure_principal_user_by_email", skip_all)]
pub async fn ensure_principal_user_by_email(
    email: Email,
    user_fetching_repo: Box<&dyn UserFetching>,
    user_registration_repo: Box<&dyn UserRegistration>,
    user_updating_repo: Box<&dyn UserUpdating>,
) -> Result<User, MappedErrors> {
    match user_fetching_repo
        .get_not_redacted_user_by_email(email.clone())
        .await?
    {
        FetchResponseKind::Found(user) if user.is_principal() => Ok(user),
        FetchResponseKind::Found(user) => {
            upgrade_to_principal(user, user_updating_repo).await
        }
        FetchResponseKind::NotFound(_) => {
            create_principal_user(email, user_registration_repo).await
        }
    }
}

async fn upgrade_to_principal(
    user: User,
    user_updating_repo: Box<&dyn UserUpdating>,
) -> Result<User, MappedErrors> {
    let mut updated = user.clone();
    let updated = updated.with_principal(true);

    match user_updating_repo.update(updated).await? {
        UpdatingResponseKind::Updated(user) => Ok(user),
        UpdatingResponseKind::NotUpdated(_, msg) => {
            use_case_err(format!("Failed to upgrade user to principal: {msg}"))
                .as_error()
        }
    }
}

/// The password hash is a random sentinel: users created this way never
/// need a password.
async fn create_principal_user(
    email: Email,
    user_registration_repo: Box<&dyn UserRegistration>,
) -> Result<User, MappedErrors> {
    let mut new_user = User::new(
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
    let new_user = new_user.with_principal(true);

    match user_registration_repo.get_or_create(new_user).await? {
        GetOrCreateResponseKind::Created(user) => Ok(user),
        GetOrCreateResponseKind::NotCreated(user, _) => Ok(user),
    }
}
