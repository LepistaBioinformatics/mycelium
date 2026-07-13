use crate::domain::{
    dtos::instance_settings::STAFF_BOOTSTRAP_KEY,
    entities::InstanceSettingsFetching,
};

use mycelium_base::{entities::FetchResponseKind, utils::errors::MappedErrors};

/// `true` while the staff bootstrap is still unclaimed, `false` once the
/// `STAFF_BOOTSTRAP_KEY` entry exists.
///
/// Called once at gateway boot. The generalized `instance_settings` table
/// has nothing to pre-create -- row *presence* under this key is itself the
/// "claimed" signal, so this is fetch-only (see feature staff-bootstrap
/// design.md §1 DEC-5/DEC-6).
#[tracing::instrument(name = "staff_bootstrap_is_pending", skip_all)]
pub async fn staff_bootstrap_is_pending(
    instance_settings_fetching_repo: Box<&dyn InstanceSettingsFetching>,
) -> Result<bool, MappedErrors> {
    match instance_settings_fetching_repo
        .get(STAFF_BOOTSTRAP_KEY.to_string())
        .await?
    {
        FetchResponseKind::NotFound(_) => Ok(true),
        FetchResponseKind::Found(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use chrono::Local;
    use mycelium_base::entities::FetchResponseKind;

    struct StubFetching {
        response: FetchResponseKind<
            crate::domain::dtos::instance_settings::InstanceSetting,
            (),
        >,
    }

    #[async_trait]
    impl InstanceSettingsFetching for StubFetching {
        async fn get(
            &self,
            _: String,
        ) -> Result<
            FetchResponseKind<
                crate::domain::dtos::instance_settings::InstanceSetting,
                (),
            >,
            MappedErrors,
        > {
            Ok(self.response.clone())
        }
    }

    #[tokio::test]
    async fn returns_true_when_not_found() -> Result<(), MappedErrors> {
        let repo = StubFetching {
            response: FetchResponseKind::NotFound(None),
        };

        let result = staff_bootstrap_is_pending(Box::new(
            &repo as &dyn InstanceSettingsFetching,
        ))
        .await?;

        assert!(result);

        Ok(())
    }

    #[tokio::test]
    async fn returns_false_when_found() -> Result<(), MappedErrors> {
        let repo = StubFetching {
            response: FetchResponseKind::Found(
                crate::domain::dtos::instance_settings::InstanceSetting {
                    key: crate::domain::dtos::instance_settings::STAFF_BOOTSTRAP_KEY.to_string(),
                    value: serde_json::json!({}),
                    created_by: None,
                    updated_by: None,
                    created: Local::now(),
                    updated: None,
                },
            ),
        };

        let result = staff_bootstrap_is_pending(Box::new(
            &repo as &dyn InstanceSettingsFetching,
        ))
        .await?;

        assert!(!result);

        Ok(())
    }
}
