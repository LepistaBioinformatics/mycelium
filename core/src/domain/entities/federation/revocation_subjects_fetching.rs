use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Emails whose asserted grants a local change removes (FED-22)
#[async_trait]
pub trait RevocationSubjectsFetching: Interface + Send + Sync {
    /// The account's own users and its guests
    async fn emails_of_account(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<String>, MappedErrors>;

    /// Guests holding the role
    async fn emails_of_guest_role(
        &self,
        guest_role_id: Uuid,
    ) -> Result<Vec<String>, MappedErrors>;
}
