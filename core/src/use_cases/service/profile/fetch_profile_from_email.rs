use crate::domain::{
    dtos::{email::Email, profile::Profile},
    entities::ProfileFetching,
};

use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};

/// Fetch the user profile from email address.
pub async fn fetch_profile_from_email(
    email: Email,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
) -> Result<FetchResponseKind<Profile, Email>, MappedErrors> {
    profile_fetching_repo.get(email).await
}
