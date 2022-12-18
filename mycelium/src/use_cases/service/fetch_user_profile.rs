use crate::domain::{
    dtos::{email::EmailDTO, profile::ProfileDTO},
    entities::service::profile_fetching::ProfileFetching,
};

use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};
use uuid::Uuid;

/// Fetch the user profile from email address.
pub async fn fetch_profile_from_email(
    email: EmailDTO,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
) -> Result<FetchResponseKind<ProfileDTO, Uuid>, MappedErrors> {
    profile_fetching_repo.get(email).await
}
