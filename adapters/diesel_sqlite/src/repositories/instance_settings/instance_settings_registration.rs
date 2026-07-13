use super::shared::map_instance_settings_row_to_dto;
use crate::{
    config::SqliteDbPoolProvider,
    models::instance_settings::InstanceSettingsRow,
    schema::instance_settings as instance_settings_model,
    types::{json_to_text, naive_timestamp_to_text},
};

use async_trait::async_trait;
use chrono::Local;
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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

        let value_text = json_to_text(&value)?;
        let created = naive_timestamp_to_text(&Local::now().naive_utc());
        let created_by_text =
            created_by.map(|m| serde_json::to_string(&m).unwrap());

        // Atomic claim -- whichever caller wins this insert wins the key.
        // Everyone else falls through to the SELECT below and observes the
        // winner's row.
        let inserted = diesel::insert_into(instance_settings_model::table)
            .values((
                instance_settings_model::key.eq(&key),
                instance_settings_model::value.eq(&value_text),
                instance_settings_model::created_by.eq(&created_by_text),
                instance_settings_model::created.eq(created),
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

        let settings = map_instance_settings_row_to_dto(row)?;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        repositories::instance_settings::InstanceSettingsFetchingSqlDbRepository,
        test_support::setup_temp_db,
    };
    use myc_core::domain::entities::InstanceSettingsFetching;
    use mycelium_base::entities::FetchResponseKind;
    use uuid::Uuid;

    #[tokio::test]
    async fn instance_settings_lifecycle_round_trips_through_sqlite(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = InstanceSettingsRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = InstanceSettingsFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let key = "staff_bootstrap".to_string();
        let value = serde_json::json!({});
        let user_id = Uuid::new_v4();
        let created_by =
            WrittenBy::new_from_user_with_email(user_id, "staff@example.com");

        // First call creates the row -- this is the "claim".
        let created = match registration
            .get_or_create(key.clone(), value.clone(), Some(created_by))
            .await?
        {
            GetOrCreateResponseKind::Created(settings) => settings,
            GetOrCreateResponseKind::NotCreated(..) => {
                panic!("expected the row to be created")
            }
        };
        assert_eq!(created.value, value);
        assert_eq!(created.created_by.unwrap().id, Some(user_id));

        // Idempotent: a second attempt to claim the same key loses the
        // race and does not overwrite the winner's created_by.
        let other_claimant = WrittenBy::new_from_user_with_email(
            Uuid::new_v4(),
            "loser@example.com",
        );
        let not_created = registration
            .get_or_create(
                key.clone(),
                serde_json::json!({}),
                Some(other_claimant),
            )
            .await?;
        assert!(matches!(
            not_created,
            GetOrCreateResponseKind::NotCreated(..)
        ));

        // Fetch confirms the winner's created_by survived.
        let found = match fetching.get(key.clone()).await? {
            FetchResponseKind::Found(settings) => settings,
            FetchResponseKind::NotFound(_) => {
                panic!("expected the row to be found")
            }
        };
        assert_eq!(found.created_by.unwrap().id, Some(user_id));
        assert!(found.updated_by.is_none());

        // A different key is independent and still unclaimed.
        let unrelated = fetching.get("some_other_setting".to_string()).await?;
        assert!(matches!(unrelated, FetchResponseKind::NotFound(_)));

        Ok(())
    }
}
