use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            email::Email,
            user::Provider,
            webhook::{AccountPropagationWebHookResponse, HookTarget},
        },
        entities::{
            AccountRegistration, AccountTypeRegistration, UserFetching,
            WebHookFetching,
        },
    },
    use_cases::roles::shared::{
        account_type::get_or_create_default_account_types,
        webhook::{default_actions::WebHookDefaultAction, dispatch_webhooks},
    },
};

use clean_base::{
    entities::{
        FetchManyResponseKind, FetchResponseKind, GetOrCreateResponseKind,
    },
    utils::errors::{factories::use_case_err, MappedErrors},
};

/// Create a default account.
///
/// Default accounts are used to mirror human users. Such accounts should not be
/// flagged as `subscription`.
///
/// This function are called when a new user start into the system. The
/// account-creation method also insert a new user into the database and set the
/// default role as `default-user`.
pub async fn create_default_account(
    email: Email,
    account_name: String,
    user_fetching_repo: Box<&dyn UserFetching>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    webhook_fetching_repo: Box<&dyn WebHookFetching>,
) -> Result<AccountPropagationWebHookResponse, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to fetch user from database
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo.get(None, Some(email), None).await? {
        FetchResponseKind::NotFound(_) => {
            return use_case_err("User not found".to_string()).as_error();
        }
        FetchResponseKind::Found(user) => user,
    };

    if let Some(Provider::Internal(_)) = user.provider() {
        if !user.is_active {
            return use_case_err("User is not active".to_string()).as_error();
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Get or Create default account type
    //
    // Get or create the default account-type.
    // ? -----------------------------------------------------------------------

    let account_type = match get_or_create_default_account_types(
        AccountTypeEnum::Standard,
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
    // ? Register the account
    //
    // The account are registered using the already created user.
    // ? -----------------------------------------------------------------------

    let account = match account_registration_repo
        .get_or_create(
            Account::new(account_name, user, account_type),
            true,
            false,
        )
        .await?
    {
        GetOrCreateResponseKind::Created(account) => account,
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(format!("Account not created: {msg}"))
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Dispatch associated webhooks
    // ? -----------------------------------------------------------------------

    let target_hooks = match webhook_fetching_repo
        .list(
            Some(WebHookDefaultAction::CreateDefaultUserAccount.to_string()),
            Some(HookTarget::Account),
        )
        .await?
    {
        FetchManyResponseKind::NotFound => None,
        FetchManyResponseKind::Found(records) => Some(records),
        FetchManyResponseKind::FoundPaginated(paginated_records) => {
            Some(paginated_records.records)
        }
    };

    let propagation_responses = match target_hooks {
        None => None,
        Some(hooks) => dispatch_webhooks(hooks, account.to_owned()).await,
    };

    Ok(AccountPropagationWebHookResponse {
        account,
        propagation_responses,
    })
}
