use crate::domain::dtos::federation::{
    ActiveTranslation, GrantTranslationRule, TranslationTarget,
};

use async_trait::async_trait;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;
use uuid::Uuid;

/// Grant translation rules, as staff and the request path read them
#[async_trait]
pub trait GrantTranslationRead: Interface + Send + Sync {
    /// Every rule, or the rules of one peer
    async fn list(
        &self,
        peer_id: Option<Uuid>,
    ) -> Result<Vec<GrantTranslationRule>, MappedErrors>;

    /// The enabled rules of these peers, with their targets as they are now;
    /// a rule whose account is deleted or has no tenant yields nothing
    async fn list_active_for_peers(
        &self,
        peer_ids: &[Uuid],
    ) -> Result<Vec<ActiveTranslation>, MappedErrors>;

    /// The account and guest role a new rule may target: both exist, the
    /// account belongs to a tenant, is not deleted and is not a system
    /// account (FED-49, I-3); `None` otherwise
    async fn target(
        &self,
        account_id: Uuid,
        guest_role_id: Uuid,
    ) -> Result<Option<TranslationTarget>, MappedErrors>;
}
