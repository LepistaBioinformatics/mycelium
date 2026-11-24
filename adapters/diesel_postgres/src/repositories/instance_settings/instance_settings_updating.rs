use super::shared::map_instance_settings_row_to_dto;
use crate::{
    models::{config::DbPoolProvider, instance_settings::InstanceSettingsRow},
    schema::instance_settings as instance_settings_model,
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{prelude::*, upsert::excluded};
use myc_core::domain::{
    dtos::{instance_settings::InstanceSetting, written_by::WrittenBy},
    entities::InstanceSettingsUpdating,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, UpdatingResponseKind},
    utils::errors::{fetching_err, updating_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;

#[derive(Component)]
#[shaku(interface = InstanceSettingsUpdating)]
pub struct InstanceSettingsUpdatingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl InstanceSettingsUpdating for InstanceSettingsUpdatingSqlDbRepository {
    #[tracing::instrument(name = "instance_settings_set", skip_all)]
    async fn set(
        &self,
        key: String,
        value: serde_json::Value,
        updated_by: Option<WrittenBy>,
    ) -> Result<UpdatingResponseKind<InstanceSetting>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            updating_err(format!("Failed to get DB connection: {}", e))
        })?;

        let written_by = updated_by
            .map(serde_json::to_value)
            .transpose()
            .map_err(|e| {
                updating_err(format!("Failed to serialize updated_by: {}", e))
            })?;

        // A new key records the writer as its creator; an existing key keeps
        // its creator and records the writer and the time of the update.
        let row = diesel::insert_into(instance_settings_model::table)
            .values((
                instance_settings_model::key.eq(&key),
                instance_settings_model::value.eq(&value),
                instance_settings_model::created_by.eq(&written_by),
            ))
            .on_conflict(instance_settings_model::key)
            .do_update()
            .set((
                instance_settings_model::value
                    .eq(excluded(instance_settings_model::value)),
                instance_settings_model::updated_by.eq(&written_by),
                instance_settings_model::updated
                    .eq(Some(Local::now().naive_utc())),
            ))
            .returning(InstanceSettingsRow::as_returning())
            .get_result::<InstanceSettingsRow>(conn)
            .map_err(|e| {
                updating_err(format!(
                    "Unexpected error detected on setting instance settings: {}",
                    e
                ))
            })?;

        Ok(UpdatingResponseKind::Updated(
            map_instance_settings_row_to_dto(row),
        ))
    }

    #[tracing::instrument(name = "instance_settings_list", skip_all)]
    async fn list(
        &self,
    ) -> Result<FetchManyResponseKind<InstanceSetting>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
        })?;

        let records = instance_settings_model::table
            .order(instance_settings_model::key.asc())
            .select(InstanceSettingsRow::as_select())
            .load::<InstanceSettingsRow>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Unexpected error detected on listing instance settings: {}",
                    e
                ))
            })?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(
            records
                .into_iter()
                .map(map_instance_settings_row_to_dto)
                .collect(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::DbPool;

    use diesel::{
        connection::SimpleConnection,
        r2d2::{ConnectionManager, Pool},
        PgConnection,
    };
    use std::sync::Once;
    use uuid::Uuid;

    /// Same variable as the other live tests of this crate. Without it every
    /// test below reports the skip and passes, so `cargo test --workspace`
    /// stays green on a machine with no Postgres.
    const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

    /// Own schema, so these tests never race the other live tests.
    const SCHEMA: &str = "myc_instance_settings_test";

    /// `instance_settings` exactly as `up.sql` creates it.
    const TABLE: &str = "
        CREATE TABLE instance_settings (
            key VARCHAR(255) PRIMARY KEY,
            value JSONB NOT NULL,
            created_by JSONB DEFAULT '{}'::JSONB,
            updated_by JSONB DEFAULT '{}'::JSONB,
            created TIMESTAMPTZ DEFAULT now(),
            updated TIMESTAMPTZ DEFAULT NULL
        );";

    static PREPARE: Once = Once::new();

    struct TestPool(DbPool);

    impl DbPoolProvider for TestPool {
        fn get_pool(&self) -> DbPool {
            self.0.clone()
        }
    }

    fn live_repository() -> Option<InstanceSettingsUpdatingSqlDbRepository> {
        let Ok(url) = std::env::var(DATABASE_URL_VAR) else {
            eprintln!(
                "skipping: set {DATABASE_URL_VAR} to run the live instance \
                 settings tests"
            );
            return None;
        };

        let separator = if url.contains('?') { '&' } else { '?' };
        let url = format!("{url}{separator}options=-csearch_path%3D{SCHEMA}");
        let pool = Pool::builder()
            .max_size(4)
            .build(ConnectionManager::<PgConnection>::new(url))
            .expect("failed to build the test pool");

        PREPARE.call_once(|| {
            pool.get()
                .expect("failed to take a test connection")
                .batch_execute(&format!(
                    "CREATE SCHEMA IF NOT EXISTS {SCHEMA};
                     DROP TABLE IF EXISTS instance_settings;
                     {TABLE}"
                ))
                .expect("failed to prepare the test schema");
        });

        Some(InstanceSettingsUpdatingSqlDbRepository {
            db_config: Arc::new(TestPool(pool)),
        })
    }

    /// A key no other test (nor a previous run) uses.
    fn unique_key() -> String {
        format!("federation.test.{}", Uuid::new_v4().simple())
    }

    fn writer(email: &str) -> Option<WrittenBy> {
        Some(WrittenBy::new_from_user_with_email(Uuid::new_v4(), email))
    }

    fn updated(
        response: UpdatingResponseKind<InstanceSetting>,
    ) -> InstanceSetting {
        match response {
            UpdatingResponseKind::Updated(setting) => setting,
            UpdatingResponseKind::NotUpdated(_, reason) => {
                panic!("set was not applied: {reason}")
            }
        }
    }

    #[tokio::test]
    async fn set_creates_an_absent_key() {
        let Some(repository) = live_repository() else {
            return;
        };
        let key = unique_key();

        let setting = updated(
            repository
                .set(
                    key.clone(),
                    serde_json::json!(true),
                    writer("a@example.com"),
                )
                .await
                .unwrap(),
        );

        assert_eq!(setting.key, key);
        assert_eq!(setting.value, serde_json::json!(true));
        assert!(setting.created_by.is_some());
        assert!(setting.updated.is_none());
    }

    #[tokio::test]
    async fn set_replaces_the_value_and_records_the_update() {
        let Some(repository) = live_repository() else {
            return;
        };
        let key = unique_key();

        let first = updated(
            repository
                .set(
                    key.clone(),
                    serde_json::json!(false),
                    writer("a@example.com"),
                )
                .await
                .unwrap(),
        );
        let second_writer = writer("b@example.com");
        let second = updated(
            repository
                .set(
                    key.clone(),
                    serde_json::json!(true),
                    second_writer.clone(),
                )
                .await
                .unwrap(),
        );

        assert_eq!(second.value, serde_json::json!(true));
        assert_eq!(second.created, first.created);
        assert!(second.updated.is_some());
        assert_eq!(
            serde_json::to_value(&second.created_by).unwrap(),
            serde_json::to_value(&first.created_by).unwrap()
        );
        assert_eq!(
            serde_json::to_value(&second.updated_by).unwrap(),
            serde_json::to_value(&second_writer).unwrap()
        );
    }

    #[tokio::test]
    async fn list_includes_a_set_key_with_its_value() {
        let Some(repository) = live_repository() else {
            return;
        };
        let key = unique_key();

        repository
            .set(key.clone(), serde_json::json!({"mode": "peers"}), None)
            .await
            .unwrap();

        let FetchManyResponseKind::Found(settings) =
            repository.list().await.unwrap()
        else {
            panic!("list must find the key just set");
        };

        let listed = settings
            .iter()
            .find(|setting| setting.key == key)
            .expect("the key just set is listed");

        assert_eq!(listed.value, serde_json::json!({"mode": "peers"}));
    }
}
