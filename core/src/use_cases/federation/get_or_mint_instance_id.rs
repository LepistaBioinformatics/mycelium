use crate::domain::{
    dtos::instance_settings::FEDERATION_INSTANCE_ID_KEY,
    entities::InstanceSettingsRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use uuid::Uuid;

/// The federation identity of this instance
///
/// Offers a fresh UUID v4 under `FEDERATION_INSTANCE_ID_KEY` with an atomic
/// claim. The first call ever persists it; every later call, and the loser of
/// a concurrent first boot, gets the stored id back. The id never changes for
/// the life of the database (FED-01).
///
#[tracing::instrument(name = "get_or_mint_instance_id", skip_all)]
pub async fn get_or_mint_instance_id(
    instance_settings_registration_repo: Box<&dyn InstanceSettingsRegistration>,
) -> Result<Uuid, MappedErrors> {
    let candidate = Uuid::new_v4();

    let stored = match instance_settings_registration_repo
        .get_or_create(
            FEDERATION_INSTANCE_ID_KEY.to_string(),
            serde_json::json!(candidate.to_string()),
            None,
        )
        .await?
    {
        GetOrCreateResponseKind::Created(setting) => setting,
        GetOrCreateResponseKind::NotCreated(setting, _) => setting,
    };

    stored
        .value
        .as_str()
        .and_then(|value| Uuid::parse_str(value).ok())
        .ok_or_else(|| {
            execution_err(format!(
                "instance setting {FEDERATION_INSTANCE_ID_KEY} does not hold \
                 a UUID"
            ))
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{
        instance_settings::{InstanceSetting, FEDERATION_INSTANCE_ID_KEY},
        written_by::WrittenBy,
    };

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::GetOrCreateResponseKind;
    use std::sync::Mutex;

    /// Behaves like the Postgres claim: the first value offered for the key
    /// wins, every later call gets the stored row back
    #[derive(Default)]
    struct ClaimStore {
        stored: Mutex<Option<serde_json::Value>>,
    }

    impl ClaimStore {
        fn holding(value: serde_json::Value) -> Self {
            Self {
                stored: Mutex::new(Some(value)),
            }
        }
    }

    fn setting(value: serde_json::Value) -> InstanceSetting {
        InstanceSetting {
            key: FEDERATION_INSTANCE_ID_KEY.to_string(),
            value,
            created_by: None,
            updated_by: None,
            created: Local::now(),
            updated: None,
        }
    }

    #[async_trait]
    impl InstanceSettingsRegistration for ClaimStore {
        async fn get_or_create(
            &self,
            key: String,
            value: serde_json::Value,
            _: Option<WrittenBy>,
        ) -> Result<GetOrCreateResponseKind<InstanceSetting>, MappedErrors>
        {
            assert_eq!(key, FEDERATION_INSTANCE_ID_KEY);

            let mut stored = self.stored.lock().unwrap();

            match stored.clone() {
                Some(existing) => Ok(GetOrCreateResponseKind::NotCreated(
                    setting(existing),
                    "already exists".to_string(),
                )),
                None => {
                    *stored = Some(value.clone());
                    Ok(GetOrCreateResponseKind::Created(setting(value)))
                }
            }
        }
    }

    async fn mint(store: &ClaimStore) -> Result<Uuid, MappedErrors> {
        get_or_mint_instance_id(Box::new(
            store as &dyn InstanceSettingsRegistration,
        ))
        .await
    }

    #[tokio::test]
    async fn first_call_mints_and_persists_a_v4_id() {
        let store = ClaimStore::default();

        let id = mint(&store).await.unwrap();

        assert_eq!(id.get_version_num(), 4);
        assert_eq!(
            *store.stored.lock().unwrap(),
            Some(serde_json::json!(id.to_string()))
        );
    }

    #[tokio::test]
    async fn later_calls_return_the_same_id() {
        let store = ClaimStore::default();

        let first = mint(&store).await.unwrap();
        let second = mint(&store).await.unwrap();

        assert_eq!(first, second);
    }

    #[tokio::test]
    async fn the_loser_of_a_concurrent_claim_returns_the_winners_id() {
        let winner = Uuid::new_v4();
        let store = ClaimStore::holding(serde_json::json!(winner.to_string()));

        assert_eq!(mint(&store).await.unwrap(), winner);
    }

    #[tokio::test]
    async fn a_stored_value_that_is_not_a_uuid_is_an_error() {
        let store = ClaimStore::holding(serde_json::json!({"id": 1}));

        assert!(mint(&store).await.is_err());
    }
}
