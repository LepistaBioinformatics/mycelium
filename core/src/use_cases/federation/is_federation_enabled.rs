use crate::domain::{
    dtos::instance_settings::FEDERATION_ENABLED_KEY,
    entities::InstanceSettingsFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// Whether the runtime federation switch is on
///
/// Off by default: no row, or any value other than JSON `true`, is off
/// (FED-06).
///
#[tracing::instrument(name = "is_federation_enabled", skip_all)]
pub async fn is_federation_enabled(
    instance_settings_fetching_repo: Box<&dyn InstanceSettingsFetching>,
) -> Result<bool, MappedErrors> {
    match instance_settings_fetching_repo
        .get(FEDERATION_ENABLED_KEY.to_string())
        .await?
    {
        FetchResponseKind::Found(setting) => Ok(setting.value == true),
        FetchResponseKind::NotFound(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::instance_settings::InstanceSetting;

    use async_trait::async_trait;
    use chrono::Local;

    struct Stored(Option<serde_json::Value>);

    #[async_trait]
    impl InstanceSettingsFetching for Stored {
        async fn get(
            &self,
            key: String,
        ) -> Result<FetchResponseKind<InstanceSetting, ()>, MappedErrors>
        {
            assert_eq!(key, FEDERATION_ENABLED_KEY);

            Ok(match &self.0 {
                None => FetchResponseKind::NotFound(None),
                Some(value) => FetchResponseKind::Found(InstanceSetting {
                    key,
                    value: value.clone(),
                    created_by: None,
                    updated_by: None,
                    created: Local::now(),
                    updated: None,
                }),
            })
        }
    }

    async fn enabled(stored: Option<serde_json::Value>) -> bool {
        let repo = Stored(stored);

        is_federation_enabled(Box::new(&repo as &dyn InstanceSettingsFetching))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn off_without_a_row() {
        assert!(!enabled(None).await);
    }

    #[tokio::test]
    async fn on_only_for_json_true() {
        assert!(enabled(Some(serde_json::json!(true))).await);
        assert!(!enabled(Some(serde_json::json!(false))).await);
        assert!(!enabled(Some(serde_json::json!("true"))).await);
    }
}
