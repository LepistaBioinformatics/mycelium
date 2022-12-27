use crate::domain::{
    dtos::{email::EmailDTO, profile::ProfileDTO},
    entities::ProfileFetching,
};

use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};

/// Fetch the user profile from email address.
pub async fn fetch_profile_from_email(
    email: EmailDTO,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
) -> Result<FetchResponseKind<ProfileDTO, EmailDTO>, MappedErrors> {
    profile_fetching_repo.get(email).await
}
