use crate::domain::dtos::{
    federation::{GrantTranslationRule, NewGrantTranslationRule},
    written_by::WrittenBy,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;
use uuid::Uuid;

/// Changes to grant translation rules (FED-48)
#[async_trait]
pub trait GrantTranslationWrite: Interface + Send + Sync {
    async fn create(
        &self,
        rule: NewGrantTranslationRule,
        created_by: Option<WrittenBy>,
    ) -> Result<GrantTranslationRule, MappedErrors>;

    /// The rule as changed, or `None` for an unknown rule
    async fn set_enabled(
        &self,
        rule_id: Uuid,
        enabled: bool,
        updated_by: Option<WrittenBy>,
    ) -> Result<Option<GrantTranslationRule>, MappedErrors>;

    async fn delete(
        &self,
        rule_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors>;
}
