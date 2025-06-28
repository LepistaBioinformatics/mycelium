use crate::{
    domain::{
        actors::SystemActor,
        dtos::{
            account_type::AccountType,
            profile::Profile,
            webhook::{PayloadId, WebHookTrigger},
        },
        entities::{AccountDeletion, WebHookRegistration},
    },
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::DeletionResponseKind, utils::errors::MappedErrors,
};
use serde_json::json;
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "delete_subscription_account",
    fields(profile_id = %profile.acc_id),
    skip(profile, account_deletion_repo, webhook_registration_repo)
)]
pub async fn delete_subscription_account(
    profile: Profile,
    tenant_id: Uuid,
    account_id: Uuid,
    account_deletion_repo: Box<&dyn AccountDeletion>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", &Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id.to_owned())
        .with_system_accounts_access()
        .with_write_access()
        .with_roles(vec![SystemActor::TenantManager])
        .get_related_accounts_or_tenant_wide_permission_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Delete account
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .soft_delete_account(
            account_id,
            AccountType::Subscription { tenant_id },
            related_accounts,
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    if let DeletionResponseKind::Deleted = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::SubscriptionAccountDeleted,
            json!({ "id": account_id }),
            PayloadId::Uuid(account_id),
            webhook_registration_repo,
        )
        .instrument(span)
        .await?;

        tracing::trace!("Side effects dispatched");
    }

    Ok(response)
}
