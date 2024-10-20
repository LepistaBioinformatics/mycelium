use super::guest_user_registration::register_guest_user;
use crate::{
    prisma::{
        guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use myc_core::domain::{
    dtos::{
        email::Email, guest::GuestUser, native_error_codes::NativeErrorCodes,
    },
    entities::GuestUserOnAccountUpdating,
};
use mycelium_base::{
    dtos::Parent,
    entities::{GetOrCreateResponseKind, UpdatingResponseKind},
    utils::errors::{deletion_err, updating_err, MappedErrors},
};
use prisma_client_rust::{prisma_errors::UnknownError, QueryError};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserOnAccountUpdating)]
pub struct GuestUserOnAccountUpdatingSqlDbRepository {}

#[async_trait]
impl GuestUserOnAccountUpdating for GuestUserOnAccountUpdatingSqlDbRepository {
    async fn update(
        &self,
        account_id: Uuid,
        old_guest_user_id: Uuid,
        new_guest_user_id: Uuid,
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
        // ? Fetch guest-user object
        // ? -------------------------------------------------------------------

        let guest_user = match client
            .guest_user()
            .find_unique(guest_user_model::id::equals(
                new_guest_user_id.to_string(),
            ))
            .exec()
            .await
        {
            Err(err) => {
                return updating_err(format!(
                    "Unable to fetch guest-user object: {err}"
                ))
                .with_exp_true()
                .as_error()
            }
            Ok(res) => match res {
                None => {
                    return updating_err(String::from(
                        "Unable to fetch guest-user object",
                    ))
                    .with_exp_true()
                    .as_error()
                }
                Some(record) => GuestUser {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    email: Email::from_string(record.email.to_owned())?,
                    guest_role: Parent::Id(
                        Uuid::parse_str(&record.guest_role_id).unwrap(),
                    ),
                    created: record.created.into(),
                    updated: match record.updated {
                        None => None,
                        Some(res) => Some(DateTime::from(res)),
                    },
                    accounts: None,
                },
            },
        };

        // ? -------------------------------------------------------------------
        // ? Perform the update
        // ? -------------------------------------------------------------------

        match client
            ._transaction()
            .run(|client| async move {
                match client
                    .guest_user_on_account()
                    .delete(
                        guest_user_on_account_model::guest_user_id_account_id(
                            old_guest_user_id.to_string(),
                            account_id.to_string(),
                        ),
                    )
                    .exec()
                    .await
                {
                    Err(err) => return Err(err),
                    _ => (),
                };

                match register_guest_user(&client, guest_user, account_id).await
                {
                    Err(err) => {
                        let error = UnknownError {
                            message: err.to_string(),
                            backtrace: None,
                        };

                        Err(QueryError::Execute(error.into()))
                    }
                    Ok(res) => Ok(res),
                }
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
            Ok(res) => match res {
                GetOrCreateResponseKind::NotCreated(guest_user, msg) => {
                    return updating_err(format!(
                        "Unexpected error on update guest user ({}): {}",
                        guest_user.email.get_email(),
                        msg
                    ))
                    .with_code(NativeErrorCodes::MYC00003)
                    .as_error()
                }
                GetOrCreateResponseKind::Created(guest_user) => {
                    Ok(UpdatingResponseKind::Updated(guest_user))
                }
            },
        }
    }
}
