use crate::domain::{
    actors::SystemActor,
    dtos::{
        profile::Profile,
        webhook::{
            WebHookExecutionEvent, WebHookExecutionStatus, WebHookTrigger,
        },
    },
    entities::WebHookExecutionFetching,
};

use mycelium_base::{
    entities::FetchManyResponseKind, utils::errors::MappedErrors,
};

/// List webhook execution events so super users can check the dispatcher.
/// Same access rule as listing webhooks.
#[tracing::instrument(
    name = "list_webhook_execution_events",
    skip(profile, execution_fetching_repo)
)]
pub async fn list_webhook_execution_events(
    profile: Profile,
    trigger: Option<WebHookTrigger>,
    status: Option<WebHookExecutionStatus>,
    page_size: Option<i32>,
    skip: Option<i32>,
    execution_fetching_repo: Box<&dyn WebHookExecutionFetching>,
) -> Result<FetchManyResponseKind<WebHookExecutionEvent>, MappedErrors> {
    profile
        .with_system_accounts_access()
        .with_read_access()
        .with_roles(vec![SystemActor::SystemManager.to_string()])
        .get_ids_or_error()?;

    execution_fetching_repo
        .list_events(trigger, status, page_size, skip)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        dtos::{
            guest_role::Permission,
            profile::{LicensedResource, LicensedResources},
        },
        entities::MockWebHookExecutionFetching,
    };
    use uuid::Uuid;

    fn profile(resources: Option<LicensedResources>) -> Profile {
        Profile::new(
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
            resources,
            None,
        )
    }

    fn system_manager() -> Profile {
        let account_id = Uuid::new_v4();
        profile(Some(LicensedResources::Records(vec![LicensedResource {
            acc_id: account_id,
            tenant_id: Uuid::new_v4(),
            role_id: Uuid::new_v4(),
            acc_name: "System manager".to_string(),
            sys_acc: true,
            role: SystemActor::SystemManager.to_string(),
            perm: Permission::Read,
            verified: true,
            permit_flags: None,
            deny_flags: None,
        }])))
    }

    #[tokio::test]
    async fn system_manager_lists_events_with_filters(
    ) -> Result<(), MappedErrors> {
        let mut repo = MockWebHookExecutionFetching::new();
        repo.expect_list_events()
            .withf(|trigger, status, page_size, skip| {
                *trigger == Some(WebHookTrigger::UserAccountCreated)
                    && *status == Some(WebHookExecutionStatus::Failed)
                    && *page_size == Some(10)
                    && *skip == Some(20)
            })
            .times(1)
            .returning(|_, _, _, _| Ok(FetchManyResponseKind::NotFound));

        let result = list_webhook_execution_events(
            system_manager(),
            Some(WebHookTrigger::UserAccountCreated),
            Some(WebHookExecutionStatus::Failed),
            Some(10),
            Some(20),
            Box::new(&repo),
        )
        .await?;

        assert!(matches!(result, FetchManyResponseKind::NotFound));
        Ok(())
    }

    #[tokio::test]
    async fn users_without_the_role_cannot_list_events() {
        let mut repo = MockWebHookExecutionFetching::new();
        repo.expect_list_events().times(0);

        let result = list_webhook_execution_events(
            profile(None),
            None,
            None,
            None,
            None,
            Box::new(&repo),
        )
        .await;

        assert!(result.is_err());
    }
}
