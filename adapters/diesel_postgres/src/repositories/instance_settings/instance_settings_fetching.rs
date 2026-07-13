use super::shared::map_instance_settings_row_to_dto;
use crate::{
    models::{config::DbPoolProvider, instance_settings::InstanceSettingsRow},
    schema::instance_settings as instance_settings_model,
};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::instance_settings::InstanceSetting,
    entities::InstanceSettingsFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = InstanceSettingsFetching)]
pub struct InstanceSettingsFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl InstanceSettingsFetching for InstanceSettingsFetchingSqlDbRepository {
    #[tracing::instrument(name = "instance_settings_get", skip_all)]
    async fn get(
        &self,
        key: String,
    ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
        })?;

        let row = instance_settings_model::table
            .filter(instance_settings_model::key.eq(key))
            .select(InstanceSettingsRow::as_select())
            .first::<InstanceSettingsRow>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!(
                    "Unexpected error detected on fetching instance settings: {}",
                    e
                ))
            })?;

        let Some(row) = row else {
            return Ok(FetchResponseKind::NotFound(None));
        };

        Ok(FetchResponseKind::Found(map_instance_settings_row_to_dto(
            row,
        )))
    }
}
