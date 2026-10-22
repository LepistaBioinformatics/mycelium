use crate::domain::{
    actors::SystemActor,
    dtos::{profile::Profile, webhook::WebHookExecutionSummary},
    entities::WebHookExecutionFetching,
};

use chrono::{DateTime, Local};
use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// Count webhook execution events by trigger and status. Same access rule
/// as listing webhooks.
#[tracing::instrument(
    name = "summarize_webhook_execution_events",
    skip(profile, execution_fetching_repo)
)]
pub async fn summarize_webhook_execution_events(
    profile: Profile,
    since: Option<DateTime<Local>>,
    execution_fetching_repo: Box<&dyn WebHookExecutionFetching>,
) -> Result<FetchManyResponseKind<WebHookExecutionSummary>, MappedErrors> {
    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    execution_fetching_repo.summarize_events(since).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::entities::MockWebHookExecutionFetching;
    use uuid::Uuid;

    #[tokio::test]
    async fn users_without_the_role_cannot_summarize_events() {
        let mut repo = MockWebHookExecutionFetching::new();
        repo.expect_summarize_events().times(0);

        let profile = Profile::new(
            vec![],
            Uuid::new_v4(),
            false,
            false,
            false,
            true,
            true,
            true,
            false,
            false,
            None,
            None,
            None,
        );

        let result =
            summarize_webhook_execution_events(profile, None, Box::new(&repo))
                .await;

        assert!(result.is_err());
    }
}
