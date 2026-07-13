use super::shared::map_instance_settings_row_to_dto;
use crate::{
    models::{config::DbPoolProvider, instance_settings::InstanceSettingsRow},
    schema::instance_settings as instance_settings_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::{instance_settings::InstanceSetting, written_by::WrittenBy},
    entities::InstanceSettingsRegistration,
};
use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = InstanceSettingsRegistration)]
pub struct InstanceSettingsRegistrationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl InstanceSettingsRegistration
    for InstanceSettingsRegistrationSqlDbRepository
{
    #[tracing::instrument(name = "instance_settings_get_or_create", skip_all)]
    async fn get_or_create(
        &self,
        key: String,
        value: serde_json::Value,
        created_by: Option<WrittenBy>,
    ) -> Result<GetOrCreateResponseKind<InstanceSetting>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            creation_err(format!("Failed to get DB connection: {}", e))
        })?;

        let created_by_value =
            created_by.map(|m| serde_json::to_value(m).unwrap());

        // Atomic claim -- whichever caller wins this insert wins the key.
        // Everyone else falls through to the SELECT below and observes the
        // winner's row.
        let inserted = diesel::insert_into(instance_settings_model::table)
            .values((
                instance_settings_model::key.eq(&key),
                instance_settings_model::value.eq(&value),
                instance_settings_model::created_by.eq(&created_by_value),
            ))
            .on_conflict_do_nothing()
            .execute(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on creating instance settings: {}",
                    e
                ))
            })?;

        let row = instance_settings_model::table
            .filter(instance_settings_model::key.eq(&key))
            .select(InstanceSettingsRow::as_select())
            .first::<InstanceSettingsRow>(conn)
            .map_err(|e| {
                creation_err(format!(
                    "Unexpected error detected on fetching instance settings after create: {}",
                    e
                ))
            })?;

        let settings = map_instance_settings_row_to_dto(row);

        if inserted > 0 {
            Ok(GetOrCreateResponseKind::Created(settings))
        } else {
            Ok(GetOrCreateResponseKind::NotCreated(
                settings,
                format!(
                    "instance_settings row for key '{}' already existed",
                    key
                ),
            ))
        }
    }
}
