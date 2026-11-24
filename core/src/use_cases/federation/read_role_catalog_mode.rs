use crate::domain::{
    dtos::{
        federation::RoleCatalogMode,
        instance_settings::FEDERATION_ROLE_CATALOG_KEY,
    },
    entities::InstanceSettingsFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// The role catalog access mode; `peers` when unset or unreadable as a mode
#[tracing::instrument(name = "read_role_catalog_mode", skip_all)]
pub async fn read_role_catalog_mode(
    instance_settings_fetching_repo: Box<&dyn InstanceSettingsFetching>,
) -> Result<RoleCatalogMode, MappedErrors> {
    let FetchResponseKind::Found(setting) = instance_settings_fetching_repo
        .get(FEDERATION_ROLE_CATALOG_KEY.to_string())
        .await?
    else {
        return Ok(RoleCatalogMode::default());
    };

    Ok(serde_json::from_value(setting.value).unwrap_or_default())
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

    async fn mode(stored: Option<serde_json::Value>) -> RoleCatalogMode {
        let repo = Stored(stored);

        read_role_catalog_mode(Box::new(&repo as &dyn InstanceSettingsFetching))
            .await
            .unwrap()
    }

    #[tokio::test]
    async fn peers_unless_set_to_a_known_mode() {
        assert_eq!(mode(None).await, RoleCatalogMode::Peers);
        assert_eq!(
            mode(Some(serde_json::json!("public"))).await,
            RoleCatalogMode::Public
        );
        assert_eq!(
            mode(Some(serde_json::json!("off"))).await,
            RoleCatalogMode::Off
        );
        assert_eq!(
            mode(Some(serde_json::json!("open"))).await,
            RoleCatalogMode::Peers
        );
    }
}
