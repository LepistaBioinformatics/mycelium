use crate::domain::{
    actors::SystemActor,
    dtos::{
        guest_role::GuestRole, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::{GuestRoleFetching, GuestRoleUpdating},
};

use futures::future;
use mycelium_base::{
    entities::{FetchResponseKind, UpdatingResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

#[tracing::instrument(name = "insert_role_child", skip_all)]
pub async fn insert_role_child(
    profile: Profile,
    role_id: Uuid,
    child_id: Uuid,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_role_updating_repo: Box<&dyn GuestRoleUpdating>,
) -> Result<UpdatingResponseKind<Option<GuestRole>>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges to create role
    // ? -----------------------------------------------------------------------

    profile
        .get_default_read_write_ids_or_error(vec![SystemActor::GuestManager])?;

    // ? -----------------------------------------------------------------------
    // ? Fetch target and child roles
    //
    // This check is necessary to guarantee that the child guest-role has the
    // same role that the target role.
    //
    // ? -----------------------------------------------------------------------

    if role_id == child_id {
        return use_case_err(
            "The target role and the child role must be different",
        )
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00018)
        .as_error();
    }

    let (target_role, children_role) = future::join(
        guest_role_fetching_repo.get(role_id),
        guest_role_fetching_repo.get(child_id),
    )
    .await;

    let target_role = match target_role? {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to check target role: {}",
                role_id,
            ))
            .as_error();
        }
        FetchResponseKind::Found(role) => role.role,
    };

    let children_role = match children_role? {
        FetchResponseKind::NotFound(_) => {
            return use_case_err(format!(
                "Unable to check child role: {}",
                child_id,
            ))
            .as_error();
        }
        FetchResponseKind::Found(role) => role.role,
    };

    if target_role != children_role {
        return use_case_err(
            "The child role must have the same role as the target role",
        )
        .with_exp_true()
        .with_code(NativeErrorCodes::MYC00018)
        .as_error();
    }

    // ? -----------------------------------------------------------------------
    // ? Persist UserRole
    // ? -----------------------------------------------------------------------

    guest_role_updating_repo
        .insert_role_child(role_id, child_id)
        .await
}
