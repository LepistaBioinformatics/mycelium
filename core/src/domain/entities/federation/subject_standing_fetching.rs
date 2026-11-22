use crate::domain::dtos::federation::SubjectStanding;

use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Whether a local user may still be vouched for, read fresh (FED-19)
#[cfg_attr(test, automock)]
#[async_trait]
pub trait SubjectStandingFetching: Interface + Send + Sync {
    /// The user's standing, or `None` when the user does not exist
    async fn standing(
        &self,
        user_id: Uuid,
    ) -> Result<Option<SubjectStanding>, MappedErrors>;
}
