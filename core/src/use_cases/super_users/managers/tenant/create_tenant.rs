use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        profile::{Owner, Profile},
        tenant::Tenant,
    },
    entities::{TenantRegistration, UserFetching},
};

use mycelium_base::{
    dtos::Children,
    entities::{CreateResponseKind, FetchResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(
    name = "create_tenant",
    fields(
        profile_id = %profile.acc_id,
        owners = ?profile.owners.iter().map(|o| o.redacted_email()).collect::<Vec<_>>(),
    ),
    skip(profile, user_fetching_repo, tenant_registration_repo)
)]
pub async fn create_tenant(
    profile: Profile,
    tenant_name: String,
    tenant_description: Option<String>,
    tenant_owner_id: Uuid,
    user_fetching_repo: Box<&dyn UserFetching>,
    tenant_registration_repo: Box<&dyn TenantRegistration>,
) -> Result<CreateResponseKind<Tenant>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    profile.has_admin_privileges_or_error()?;

    // ? -----------------------------------------------------------------------
    // ? Check if the proposed tenant owner exists
    // ? -----------------------------------------------------------------------

    let user = match user_fetching_repo.get_user_by_id(tenant_owner_id).await? {
        FetchResponseKind::Found(res) => res,
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "User with ID {} not already registered",
                tenant_owner_id
            ))
            .with_code(NativeErrorCodes::MYC00009)
            .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Initialize tenant object
    // ? -----------------------------------------------------------------------

    let tenant = Tenant::new_with_owners(
        tenant_name,
        tenant_description,
        Children::Records(vec![Owner::from_user(user)?]),
    );

    // ? -----------------------------------------------------------------------
    // ? Register tenant
    // ? -----------------------------------------------------------------------

    tenant_registration_repo
        .create(tenant, format!("account-id:{}", profile.acc_id))
        .await
}
