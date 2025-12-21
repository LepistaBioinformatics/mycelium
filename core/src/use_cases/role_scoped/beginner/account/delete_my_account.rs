use crate::{
    domain::{
        dtos::{
            account_type::AccountType,
            profile::Profile,
            related_accounts::RelatedAccounts,
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
    name = "delete_my_account",
    fields(
        profile_id = %profile.acc_id,
        correspondence_id = tracing::field::Empty
    ),
    skip(profile, account_deletion_repo, webhook_registration_repo)
)]
pub async fn delete_my_account(
    profile: Profile,
    account_deletion_repo: Box<&dyn AccountDeletion>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let response = account_deletion_repo
        .soft_delete_account(
            profile.acc_id,
            AccountType::User,
            RelatedAccounts::AllowedAccounts(vec![profile.acc_id]),
        )
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    if let DeletionResponseKind::Deleted = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountDeleted,
            json!({ "id": profile.acc_id }),
            PayloadId::Uuid(profile.acc_id),
            webhook_registration_repo,
        )
        .instrument(span)
        .await?;

        tracing::trace!("Side effects dispatched");
    }

    Ok(response)
}
