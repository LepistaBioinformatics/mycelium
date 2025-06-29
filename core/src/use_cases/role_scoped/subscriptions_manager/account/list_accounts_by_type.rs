use crate::domain::{
    actors::SystemActor,
    dtos::{
        account::Account, account_type::AccountType, guest_role::Permission,
        native_error_codes::NativeErrorCodes, profile::Profile,
        related_accounts::RelatedAccounts,
    },
    entities::AccountFetching,
    utils::try_as_uuid,
};

use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{execution_err, MappedErrors},
};
use uuid::Uuid;

/// List subscription and related accounts
///
/// Get a list of available accounts given the AccountTypeEnum.
#[tracing::instrument(
    name = "list_accounts_by_type",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn list_accounts_by_type(
    profile: Profile,
    tenant_id: Option<Uuid>,
    term: Option<String>,
    is_owner_active: Option<bool>,
    is_account_active: Option<bool>,
    is_account_checked: Option<bool>,
    is_account_archived: Option<bool>,
    is_account_deleted: Option<bool>,
    account_type: Option<AccountType>,
    tag_value: Option<String>,
    page_size: Option<i32>,
    skip: Option<i32>,
    account_fetching_repo: Box<&dyn AccountFetching>,
) -> Result<FetchManyResponseKind<Account>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    //
    // Account type is required. If not provided, it is assumed that the
    // account type is a subscription account.
    //
    let account_type = match account_type.to_owned() {
        Some(t) => t,
        None => {
            let tenant_id = match tenant_id {
                Some(id) => id,
                None => return execution_err(
                    "Tenant ID is required when no account type is provided",
                )
                .with_code(NativeErrorCodes::MYC00019)
                .with_exp_true()
                .as_error(),
            };

            AccountType::Subscription { tenant_id }
        }
    };

    //
    // Check if the current account has sufficient privileges. Inclusive the
    // subscription account should be tested.
    //
    let (filtered_profile, has_tenant_wide_privileges) =
        check_user_privileges_given_account_type(
            &profile,
            Some(account_type.to_owned()),
            tenant_id,
        )?;

    let related_accounts = if has_tenant_wide_privileges {
        if let Some(tenant_id) = tenant_id {
            RelatedAccounts::HasTenantWidePrivileges(tenant_id)
        } else {
            return execution_err(
                "tenant_id is required when listing accounts by type",
            )
            .with_code(NativeErrorCodes::MYC00019)
            .with_exp_true()
            .as_error();
        }
    } else {
        filtered_profile
            .with_read_access()
            .with_roles(vec![
                SystemActor::TenantManager,
                SystemActor::SubscriptionsManager,
            ])
            .get_related_account_or_error()?
    };

    // ? -----------------------------------------------------------------------
    // ? List accounts
    // ? -----------------------------------------------------------------------

    let (updated_term, account_id) = {
        if let Some(i) = term {
            match try_as_uuid(&i) {
                Ok(id) => (None, Some(id)),
                Err(_) => (Some(i), None),
            }
        } else {
            (None, None)
        }
    };

    let (updated_tag, tag_id) = {
        if let Some(i) = tag_value {
            match try_as_uuid(&i) {
                Ok(id) => (None, Some(id)),
                Err(_) => (Some(i), None),
            }
        } else {
            (None, None)
        }
    };

    account_fetching_repo
        .list(
            related_accounts,
            updated_term,
            is_owner_active,
            is_account_active,
            is_account_checked,
            is_account_archived,
            is_account_deleted,
            tag_id,
            updated_tag,
            account_id,
            account_type,
            page_size,
            skip,
        )
        .await
}

fn check_user_privileges_given_account_type(
    profile: &Profile,
    account_type: Option<AccountType>,
    tenant_id: Option<Uuid>,
) -> Result<(Profile, bool), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    //
    // Check if desired account types are user related
    //
    // Only admin users should be able to list user related accounts.
    //
    let admin_status_needed = match account_type.to_owned() {
        Some(types) => types.is_user_account(),
        None => false,
    };

    if admin_status_needed {
        profile.has_admin_privileges_or_error()?;
    }

    let mut filtered_profile = profile.clone();

    //
    // Check if desired account types are tenant dependent
    //
    // If true, users must provide a tenant ID.
    //
    let is_tenant_dependent = match account_type.to_owned() {
        Some(account_type) => account_type.is_tenant_dependent(),
        None => false,
    };

    if is_tenant_dependent {
        if let Some(tenant_id) = tenant_id {
            //
            // Check if the profile has tenant ownership
            //
            if filtered_profile
                .on_tenant(tenant_id)
                .with_tenant_ownership_or_error(tenant_id)
                .is_ok()
            {
                return Ok((filtered_profile, true));
            }

            //
            // Check if the profile has tenant wide privileges by using the
            // tenant manager role.
            //
            if filtered_profile
                .on_tenant_as_manager(tenant_id, Permission::Read)
                .get_ids_or_error()
                .is_ok()
            {
                return Ok((filtered_profile, true));
            }

            filtered_profile = filtered_profile
                .on_tenant_as_manager(tenant_id, Permission::Read)
                .on_tenant(tenant_id);
        } else {
            return execution_err(
                "tenant_id is required when listing accounts by type",
            )
            .with_code(NativeErrorCodes::MYC00019)
            .with_exp_true()
            .as_error();
        }
    }

    //
    // Check if the desired account types are system default accounts
    //
    // If true, the related accounts should be the system default accounts.
    //
    let is_system_default_account = match account_type.to_owned() {
        Some(types) => types.is_system_default_account(),
        None => false,
    };

    if is_system_default_account {
        filtered_profile = filtered_profile.with_system_accounts_access();
    }

    Ok((filtered_profile, false))
}
