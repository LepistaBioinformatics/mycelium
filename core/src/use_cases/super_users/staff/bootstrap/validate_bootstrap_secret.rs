use crate::domain::{
    dtos::instance_settings::STAFF_BOOTSTRAP_KEY,
    entities::InstanceSettingsFetching,
};

use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use subtle::ConstantTimeEq;

/// Validate a presented bootstrap secret against the configured one, and
/// confirm the `STAFF_BOOTSTRAP_KEY` entry doesn't exist yet (i.e. bootstrap
/// is still pending).
///
/// Returns an expected/handled error (never a raw system error) for every
/// rejection reason — bootstrap disabled (no secret configured), wrong
/// secret, or already-claimed — so callers can render the same generic "not
/// available" response regardless of cause (spec SB-R8, design §7's
/// enumeration-avoidance note).
#[tracing::instrument(name = "validate_bootstrap_secret", skip_all)]
pub async fn validate_bootstrap_secret(
    presented_secret: &str,
    configured_secret: Option<&str>,
    instance_settings_fetching_repo: Box<&dyn InstanceSettingsFetching>,
) -> Result<(), MappedErrors> {
    let Some(configured_secret) = configured_secret else {
        return use_case_err("Staff bootstrap is not enabled")
            .with_exp_true()
            .as_error();
    };

    let presented_bytes = presented_secret.as_bytes();
    let configured_bytes = configured_secret.as_bytes();

    let secrets_match = presented_bytes.len() == configured_bytes.len()
        && presented_bytes.ct_eq(configured_bytes).unwrap_u8() == 1;

    if !secrets_match {
        return use_case_err("Invalid bootstrap secret")
            .with_exp_true()
            .as_error();
    }

    match instance_settings_fetching_repo
        .get(STAFF_BOOTSTRAP_KEY.to_string())
        .await?
    {
        FetchResponseKind::NotFound(_) => Ok(()),
        FetchResponseKind::Found(_) => {
            use_case_err("Staff bootstrap has already been completed")
                .with_exp_true()
                .as_error()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use async_trait::async_trait;
    use chrono::Local;

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

    fn pending_repo() -> StubFetching {
        StubFetching {
            response: FetchResponseKind::NotFound(None),
        }
    }

    fn claimed_repo() -> StubFetching {
        StubFetching {
            response: FetchResponseKind::Found(
                crate::domain::dtos::instance_settings::InstanceSetting {
                    key: STAFF_BOOTSTRAP_KEY.to_string(),
                    value: serde_json::json!({}),
                    created_by: None,
                    updated_by: None,
                    created: Local::now(),
                    updated: None,
                },
            ),
        }
    }

    #[tokio::test]
    async fn rejects_when_no_secret_is_configured() {
        let repo = pending_repo();
        let result = validate_bootstrap_secret(
            "anything",
            None,
            Box::new(&repo as &dyn InstanceSettingsFetching),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_wrong_secret() {
        let repo = pending_repo();
        let result = validate_bootstrap_secret(
            "wrong",
            Some("correct"),
            Box::new(&repo as &dyn InstanceSettingsFetching),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rejects_when_already_claimed_even_with_correct_secret() {
        let repo = claimed_repo();
        let result = validate_bootstrap_secret(
            "correct",
            Some("correct"),
            Box::new(&repo as &dyn InstanceSettingsFetching),
        )
        .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn accepts_correct_secret_while_pending() -> Result<(), MappedErrors>
    {
        let repo = pending_repo();
        validate_bootstrap_secret(
            "correct",
            Some("correct"),
            Box::new(&repo as &dyn InstanceSettingsFetching),
        )
        .await
    }
}
