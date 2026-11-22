use crate::domain::{
    dtos::federation::StoredAssertion, entities::AssertionStoreRead,
};

use mycelium_base::utils::errors::MappedErrors;

/// Every valid assertion this instance holds from its peers (D-6)
#[tracing::instrument(name = "list_stored_assertions", skip_all)]
pub async fn list_stored_assertions(
    assertion_read_repo: Box<&dyn AssertionStoreRead>,
) -> Result<Vec<StoredAssertion>, MappedErrors> {
    assertion_read_repo.list_valid(None).await
}
