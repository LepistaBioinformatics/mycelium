use crate::{
    domain::{
        dtos::{
            account::Account,
            profile::Profile,
            webhook::{PayloadId, WebHookTrigger},
        },
        entities::{AccountUpdating, WebHookRegistration},
    },
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::UpdatingResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::Instrument;
use uuid::Uuid;

/// Update the own account.
///
/// This function uses the id of the Profile to fetch and update the account
/// name, allowing only the account owner to update the account name.
#[tracing::instrument(
    name = "update_own_account_name", 
    skip_all,
    fields(correspondence_id = tracing::field::Empty),
)]
pub async fn update_own_account_name(
    profile: Profile,
    name: String,
    account_updating_repo: Box<&dyn AccountUpdating>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<UpdatingResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Update and persist account name
    // ? -----------------------------------------------------------------------

    let response = account_updating_repo
        .update_own_account_name(profile.acc_id, name)
        .await?;

    if let UpdatingResponseKind::Updated(account) = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        let account_id = account.id.ok_or_else(|| {
            use_case_err("Account ID not found".to_string()).with_exp_true()
        })?;

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::UserAccountUpdated,
            account.to_owned(),
            PayloadId::Uuid(account_id),
            webhook_registration_repo,
        )
        .instrument(span)
        .await?;

        tracing::trace!("Side effects dispatched");
    }

    Ok(response)
}
