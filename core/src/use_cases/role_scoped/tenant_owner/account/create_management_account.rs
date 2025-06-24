use crate::domain::{
    dtos::{account::Account, profile::Profile},
    entities::AccountRegistration,
};

use mycelium_base::{
    entities::GetOrCreateResponseKind, utils::errors::MappedErrors,
};
use slugify::slugify;
use uuid::Uuid;

#[tracing::instrument(
    name = "create_management_account",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(profile, account_registration_repo)
)]
pub async fn create_management_account(
    profile: Profile,
    tenant_id: Uuid,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<GetOrCreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Initialize tracing span
    // ? -----------------------------------------------------------------------

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
    // ? Return a positive response
    // ? -----------------------------------------------------------------------

    Ok(response)
}
