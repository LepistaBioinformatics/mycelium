use crate::domain::dtos::{
    instance_settings::InstanceSetting, written_by::WrittenBy,
};

use async_trait::async_trait;
use mycelium_base::{
    entities::{FetchManyResponseKind, UpdatingResponseKind},
    utils::errors::MappedErrors,
};
use shaku::Interface;

/// Update and list instance settings
///
/// Kept apart from `InstanceSettingsFetching` and
/// `InstanceSettingsRegistration` so only the backends that need it (Postgres)
/// implement it; `diesel_sqlite` is untouched.
///
#[async_trait]
pub trait InstanceSettingsUpdating: Interface + Send + Sync {
    /// Upsert: creates the row when the key is absent, otherwise replaces its
    /// `value` and records `updated_by` and `updated`. Returns the row as now
    /// stored.
    async fn set(
        &self,
        key: String,
        value: serde_json::Value,
        updated_by: Option<WrittenBy>,
    ) -> Result<UpdatingResponseKind<InstanceSetting>, MappedErrors>;

    /// Every stored setting
    async fn list(
        &self,
    ) -> Result<FetchManyResponseKind<InstanceSetting>, MappedErrors>;
}
