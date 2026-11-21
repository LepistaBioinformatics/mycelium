use crate::{
    domain::{
        dtos::{
            instance_settings::{InstanceSetting, FEDERATION_ENABLED_KEY},
            written_by::WrittenBy,
        },
        entities::InstanceSettingsUpdating,
    },
    models::FederationConfig,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Turn federation on or off at runtime (FED-06)
///
/// Enabling is refused while `[core.federation]` is absent: there would be no
/// base URL and no signing key to serve peers with (FED-02). Disabling is
/// always allowed.
///
#[tracing::instrument(name = "set_federation_enabled", skip_all)]
pub async fn set_federation_enabled(
    enabled: bool,
    config: Option<&FederationConfig>,
    updated_by: Option<WrittenBy>,
    instance_settings_updating_repo: Box<&dyn InstanceSettingsUpdating>,
) -> Result<InstanceSetting, MappedErrors> {
    if enabled && config.is_none() {
        return use_case_err(
            "federation cannot be enabled while [core.federation] is absent \
             from the configuration",
        )
        .with_exp_true()
        .as_error();
    }

    match instance_settings_updating_repo
        .set(
            FEDERATION_ENABLED_KEY.to_string(),
            serde_json::json!(enabled),
            updated_by,
        )
        .await?
    {
        UpdatingResponseKind::Updated(setting) => Ok(setting),
        UpdatingResponseKind::NotUpdated(_, reason) => {
            use_case_err(reason).as_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::instance_settings::FEDERATION_ENABLED_KEY;

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::{
        FetchManyResponseKind, UpdatingResponseKind,
    };
    use std::sync::Mutex;

    #[derive(Default)]
    struct Recorder {
        written: Mutex<Vec<(String, serde_json::Value)>>,
    }

    #[async_trait]
    impl InstanceSettingsUpdating for Recorder {
        async fn set(
            &self,
            key: String,
            value: serde_json::Value,
            _: Option<WrittenBy>,
        ) -> Result<UpdatingResponseKind<InstanceSetting>, MappedErrors>
        {
            self.written
                .lock()
                .unwrap()
                .push((key.clone(), value.clone()));

            Ok(UpdatingResponseKind::Updated(InstanceSetting {
                key,
                value,
                created_by: None,
                updated_by: None,
                created: Local::now(),
                updated: None,
            }))
        }

        async fn list(
            &self,
        ) -> Result<FetchManyResponseKind<InstanceSetting>, MappedErrors>
        {
            unreachable!()
        }
    }

    fn config() -> FederationConfig {
        serde_json::from_value(serde_json::json!({
            "baseUrl": "https://myc-01.example.com",
        }))
        .unwrap()
    }

    async fn set(
        repo: &Recorder,
        enabled: bool,
        config: Option<&FederationConfig>,
    ) -> Result<InstanceSetting, MappedErrors> {
        set_federation_enabled(
            enabled,
            config,
            None,
            Box::new(repo as &dyn InstanceSettingsUpdating),
        )
        .await
    }

    #[tokio::test]
    async fn enabling_without_the_config_section_is_refused() {
        let repo = Recorder::default();

        let error = set(&repo, true, None).await.unwrap_err().to_string();

        assert!(error.contains("core.federation"), "{error}");
        assert!(repo.written.lock().unwrap().is_empty());
    }

    #[tokio::test]
    async fn enabling_with_the_config_section_is_stored() {
        let repo = Recorder::default();

        let setting = set(&repo, true, Some(&config())).await.unwrap();

        assert_eq!(setting.value, serde_json::json!(true));
        assert_eq!(
            *repo.written.lock().unwrap(),
            [(FEDERATION_ENABLED_KEY.to_string(), serde_json::json!(true))]
        );
    }

    #[tokio::test]
    async fn disabling_is_always_allowed() {
        for config in [None, Some(config())] {
            let repo = Recorder::default();

            let setting = set(&repo, false, config.as_ref()).await.unwrap();

            assert_eq!(setting.value, serde_json::json!(false));
        }
    }
}
