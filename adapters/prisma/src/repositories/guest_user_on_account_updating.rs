use crate::{
    prisma::{
        guest_role as guest_role_model, guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use myc_core::domain::{
    dtos::{
        email::Email, guest_role::Permission, guest_user::GuestUser,
        native_error_codes::NativeErrorCodes,
    },
    entities::GuestUserOnAccountUpdating,
};
use mycelium_base::{
    dtos::Parent,
    entities::UpdatingResponseKind,
    utils::errors::{deletion_err, updating_err, MappedErrors},
};
use prisma_client_rust::{and, prisma_errors::Error, QueryError};
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserOnAccountUpdating)]
pub struct GuestUserOnAccountUpdatingSqlDbRepository {}

#[async_trait]
impl GuestUserOnAccountUpdating for GuestUserOnAccountUpdatingSqlDbRepository {
    async fn accept_invitation(
        &self,
        guest_role_name: String,
        account_id: Uuid,
        permission: Permission,
    ) -> Result<UpdatingResponseKind<GuestUser>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return deletion_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Perform the update
        // ? -------------------------------------------------------------------

        match client
            ._transaction()
            .run(|client| async move {
                let guest_user_data = match client
                    .guest_user()
                    .find_many(vec![and![
                        guest_user_model::guest_role::is(vec![
                            guest_role_model::name::equals(guest_role_name.to_owned()),
                        ]),
                        guest_user_model::guest_role::is(vec![
                            guest_role_model::permission::equals(permission.to_i32()),
                        ]),
                        guest_user_model::accounts::some(vec![
                            guest_user_on_account_model::account_id::equals(account_id.to_string()),
                        ])
                    ]])
                    .exec()
                    .await
                {
                    Err(err) => {
                        return Err(err);
                    }
                    Ok(guest) => guest,
                };

                let guest_user = match guest_user_data.len() {
                    0 => {
                        return Err(QueryError::Execute(Error::new_non_panic_with_current_backtrace(
                            format!("Guest user with role name {guest_role_name} not found"),
                        )));
                    }
                    1 => guest_user_data[0].clone(),
                    _ => {
                        return Err(QueryError::Execute(Error::new_non_panic_with_current_backtrace(
                            format!("Multiple guest users with role name {guest_role_name} found"),
                        )));
                    }
                };

                client
                    .guest_user()
                    .update(
                        guest_user_model::id::equals(guest_user.id),
                        vec![guest_user_model::was_verified::set(true)],
                    )
                    .include(guest_user_model::include!({
                        guest_role: select { id }
                    }))
                    .exec()
                    .await
            })
            .await
        {
            Err(err) => {
                return updating_err(format!(
                    "Unable to update guest-user object: {err}"
                ))
                .with_exp_true()
                .as_error()
            }
            Ok(record) => {
                Ok(UpdatingResponseKind::Updated(GuestUser::new_existing(
                    Uuid::from_str(&record.id).unwrap(),
                    Email::from_string(record.email.to_owned())?,
                    Parent::Id(Uuid::parse_str(&record.guest_role.id).unwrap()),
                    record.created.into(),
                    match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                    None,
                    record.was_verified,
                )))
            }
        }
    }
}
