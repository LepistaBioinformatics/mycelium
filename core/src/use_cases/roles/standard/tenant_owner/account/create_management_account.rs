use crate::{
    domain::{
        actors::DefaultActor,
        dtos::{
            account::{Account, AccountTypeEnum},
            profile::Profile,
        },
        entities::{
            AccountRegistration, AccountTypeRegistration, TenantFetching,
        },
    },
    use_cases::roles::shared::account_type::get_or_create_default_account_types,
};

use mycelium_base::{
    entities::{
        CreateResponseKind, FetchResponseKind, GetOrCreateResponseKind,
    },
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "create_management_account",
    fields(
        account_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.email.to_owned()).collect::<Vec<_>>(),
    ),
    skip(
        profile, 
        tenant_fetching_repo, 
        account_type_registration_repo, 
        account_registration_repo
    )
)]
pub async fn create_management_account(
    profile: Profile,
    tenant_id: Uuid,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
) -> Result<CreateResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check the user permissions
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .get_related_account_with_default_create_or_error(vec![
            DefaultActor::TenantOwner.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch tenant
    // ? -----------------------------------------------------------------------

    let tenant =
        match tenant_fetching_repo
            .get(tenant_id, related_accounts)
            .await?
        {
            FetchResponseKind::NotFound(msg) => return use_case_err(
                msg.unwrap_or(
                    "Tenant does not exist or inaccessible for the user".to_string(),
                ),
            )
            .as_error(),
            FetchResponseKind::Found(tenant) => tenant,
        };

    // ? -----------------------------------------------------------------------
    // ? Fetch account type
    //
    // Get or create the default account-type.
    // ? -----------------------------------------------------------------------

    let account_type = match get_or_create_default_account_types(
        AccountTypeEnum::Subscription,
        None,
        None,
        account_type_registration_repo,
    )
    .await?
    {
        GetOrCreateResponseKind::NotCreated(account_type, _) => account_type,
        GetOrCreateResponseKind::Created(account_type) => account_type,
    };

    // ? -----------------------------------------------------------------------
    // ? Register account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let mut unchecked_account =
        Account::new_subscription_account(
            String::new(), 
            account_type,
        ).with_id();

    unchecked_account.is_checked = true;

    unchecked_account.name = if let Some(id) = unchecked_account.id {
        format!(
            "{}/managed-by/{}", 
            tenant.tenant_string_or_error()?, 
            id.to_string()
        )
    } else {
        return use_case_err("Unable to format account name").as_error();
    };

    account_registration_repo
        .create_subscription_account(unchecked_account, tenant_id)
        .await
}
