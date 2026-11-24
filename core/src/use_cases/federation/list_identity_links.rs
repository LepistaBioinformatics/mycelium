use crate::domain::{
    dtos::federation::IdentityLink, entities::IdentityLinkRead,
};

use mycelium_base::utils::errors::MappedErrors;
use uuid::Uuid;

/// The user's links, or every link for an operator (`None`), newest first
#[tracing::instrument(name = "list_identity_links", skip_all)]
pub async fn list_identity_links(
    local_user_id: Option<Uuid>,
    link_read_repo: Box<&dyn IdentityLinkRead>,
) -> Result<Vec<IdentityLink>, MappedErrors> {
    link_read_repo.list(local_user_id).await
}
