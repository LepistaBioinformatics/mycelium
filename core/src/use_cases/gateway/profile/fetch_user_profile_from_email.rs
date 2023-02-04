use crate::domain::entities::ProfileFetching;

use clean_base::{
    entities::default_response::FetchResponseKind, utils::errors::MappedErrors,
};
use myc_http_tools::{Email, ProfilePack};

/// Fetch the account profile given the user email.
///
/// All authorization operations are based on the user email. Thus, user profile
/// information should also be required based on email.
pub async fn fetch_user_profile_from_email(
    email: Email,
    service: String,
    profile_fetching_repo: Box<dyn ProfileFetching>,
) -> Result<FetchResponseKind<ProfilePack, Email>, MappedErrors> {
    profile_fetching_repo.get(email, service).await
}
