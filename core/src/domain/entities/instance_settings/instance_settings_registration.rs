use crate::domain::dtos::{
    instance_settings::InstanceSetting, written_by::WrittenBy,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use shaku::Interface;

#[async_trait]
pub trait InstanceSettingsRegistration: Interface + Send + Sync {
    /// Atomic claim: `INSERT ... ON CONFLICT (key) DO NOTHING`. `Created`
    /// when this call inserted the row -- i.e. this call is the one that won
    /// the key; `NotCreated` when the key already existed (another
    /// replica/caller got there first). Either way the returned
    /// `InstanceSetting` reflects the row now in the DB. This is the
    /// keyed-row equivalent of a compare-and-swap: presence of the row is
    /// the state, so "claiming" a key is simply creating it. `created_by`
    /// records who performed the insert (may be email-only, e.g. before any
    /// Account exists yet) -- persisted only when this call actually wins.
    async fn get_or_create(
        &self,
        key: String,
        value: serde_json::Value,
        created_by: Option<WrittenBy>,
    ) -> Result<GetOrCreateResponseKind<InstanceSetting>, MappedErrors>;
}
