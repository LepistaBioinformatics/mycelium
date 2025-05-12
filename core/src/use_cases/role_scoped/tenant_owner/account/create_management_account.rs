use crate::{
    domain::{
        dtos::{
            account::Account,
            profile::Profile,
            webhook::{PayloadId, WebHookTrigger},
        },
        entities::{AccountRegistration, WebHookRegistration},
    },
    use_cases::support::register_webhook_dispatching_event,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use slugify::slugify;
use tracing::Instrument;
use uuid::Uuid;

#[tracing::instrument(
    name = "create_management_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, account_registration_repo, webhook_registration_repo)
)]
pub async fn create_management_account(
    profile: Profile,
    tenant_id: Uuid,
    account_registration_repo: Box<&dyn AccountRegistration>,
    webhook_registration_repo: Box<&dyn WebHookRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

    let span = tracing::Span::current();

    let correspondence_id = Uuid::new_v4();

    tracing::Span::current()
        .record("correspondence_id", &Some(correspondence_id.to_string()));

    // ? -----------------------------------------------------------------------
    // ? Check if the profile is the owner of the tenant
    // ? -----------------------------------------------------------------------

    profile.with_tenant_ownership_or_error(tenant_id)?;

    // ? -----------------------------------------------------------------------
    // ? Register account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account =
        Account::new_tenant_management_account(String::new(), tenant_id)
            .with_id();

    let name =
        format!("tid/{}/manager", tenant_id.to_string().replace("-", ""));

    unchecked_account.is_checked = true;
    unchecked_account.is_default = true;
    unchecked_account.name = name.to_owned();
    unchecked_account.slug = slugify!(&name.as_str());

    let response = account_registration_repo
        .get_or_create_tenant_management_account(unchecked_account, tenant_id)
        .await?;

    // ? -----------------------------------------------------------------------
    // ? Propagate account
    // ? -----------------------------------------------------------------------

    if let GetOrCreateResponseKind::Created(account) = response.to_owned() {
        tracing::trace!("Dispatching side effects");

        let account_id = account.id.ok_or_else(|| {
            use_case_err("Account ID not found".to_string()).with_exp_true()
        })?;

        register_webhook_dispatching_event(
            correspondence_id,
            WebHookTrigger::SubscriptionAccountCreated,
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
