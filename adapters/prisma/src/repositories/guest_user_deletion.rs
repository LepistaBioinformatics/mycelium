use crate::{
    prisma::{
        guest_user as guest_user_model,
        guest_user_on_account as guest_user_on_account_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::DeletionResponseKind,
    utils::errors::{factories::deletion_err, MappedErrors},
};
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::GuestUserDeletion,
};
use prisma_client_rust::and;
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserDeletion)]
pub struct GuestUserDeletionSqlDbRepository {}

#[async_trait]
impl GuestUserDeletion for GuestUserDeletionSqlDbRepository {
    async fn delete(
        &self,
        guest_role_id: Uuid,
        account_id: Uuid,
        email: String,
    ) -> Result<DeletionResponseKind<(Uuid, Uuid)>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return deletion_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001.as_str())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the deletion query
        // ? -------------------------------------------------------------------

        match client
            ._transaction()
            .run(|client| async move {
                let guest = match client
                    .guest_user_on_account()
                    .find_first(vec![and![
                        guest_user_on_account_model::account_id::equals(
                            account_id.to_string(),
                        ),
                        guest_user_on_account_model::guest_user::is(vec![
                            and![
                                guest_user_model::email::equals(email),
                                guest_user_model::guest_role_id::equals(
                                    guest_role_id.to_string(),
                                ),
                            ]
                        ]),
                    ]])
                    .exec()
                    .await
                {
                    Err(err) => {
                        return Err(err);
                    }
                    Ok(guest) => guest,
                };

                println!("Guest: {:?}", guest);

                let guest = match guest {
                    None => {
                        return Ok(DeletionResponseKind::NotDeleted(
                            (guest_role_id, account_id),
                            "Guest user not found".to_string(),
                        ))
                    }
                    Some(guest) => guest,
                };

                match client
                    .guest_user_on_account()
                    .delete(
                        guest_user_on_account_model::guest_user_id_account_id(
                            guest.guest_user_id.to_string(),
                            guest.account_id.to_string(),
                        ),
                    )
                    .exec()
                    .await
                {
                    Err(err) => Ok(DeletionResponseKind::NotDeleted(
                        (guest_role_id, account_id),
                        err.to_string(),
                    )),
                    Ok(_) => Ok(DeletionResponseKind::Deleted),
                }
            })
            .await
        {
            Err(err) => Ok(DeletionResponseKind::NotDeleted(
                (guest_role_id, account_id),
                err.to_string(),
            )),
            Ok(res) => Ok(res),
        }
    }
}
