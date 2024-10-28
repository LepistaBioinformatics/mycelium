use crate::{
    prisma::{
        account as account_model, guest_role as guest_role_model,
        guest_user as guest_user_model, PrismaClient,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use log::debug;
use myc_core::domain::{
    dtos::{
        email::Email, guest_user::GuestUser, native_error_codes::NativeErrorCodes,
    },
    entities::GuestUserRegistration,
};
use mycelium_base::{
    dtos::Parent,
    entities::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use prisma_client_rust::prisma_errors::query_engine::UniqueKeyViolation;
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use tracing::warn;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserRegistration)]
pub struct GuestUserRegistrationSqlDbRepository {}

#[async_trait]
impl GuestUserRegistration for GuestUserRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        guest_user: GuestUser,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        register_guest_user(client, guest_user, account_id).await

        //Ok(GetOrCreateResponseKind::Created(_guest_user))
    }
}

pub(super) async fn register_guest_user(
    client: &PrismaClient,
    guest_user: GuestUser,
    account_id: Uuid,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Try to get the guest user
    //
    // The guest user include the combination of the user email with the
    // guest-role id.
    //
    // ? -----------------------------------------------------------------------

    let _guest_user = match client
        .guest_user()
        .find_first(vec![
            guest_user_model::email::equals(
                guest_user.email.to_owned().get_email(),
            ),
            guest_user_model::guest_role_id::equals(
                match guest_user.guest_role.to_owned() {
                    Parent::Id(id) => id.to_string(),
                    Parent::Record(record) => match record.id {
                        None => {
                            // !
                            // ! Error case return
                            // !
                            return creation_err(String::from(
                                "Unable to get the guest role ID.",
                            ))
                            .as_error();
                        }
                        Some(id) => id.to_string(),
                    },
                },
            ),
        ])
        .include(guest_user_model::include!({
            guest_role: select { id }
        }))
        .exec()
        .await
    {
        // !
        // ! Error case return
        // !
        Err(err) => {
            return creation_err(format!(
                "Unexpected error on check guest user: {:?}",
                err
            ))
            .as_error()
        }
        Ok(res) => res,
    };

    debug!("_guest_user (1): {:?}", _guest_user);

    // ? -----------------------------------------------------------------------
    // ? Check if the guest user already exists
    //
    // Extract the request result case the previous step not returns an error.
    //
    // ? -----------------------------------------------------------------------

    let _guest_user = match _guest_user {
        //
        // If the fetching operation find a object, try to parse the
        // response as a GuestUser.
        //
        Some(record) => GuestUser {
            id: Some(Uuid::from_str(&record.id).unwrap()),
            email: Email::from_string(record.email)?,
            guest_role: Parent::Id(
                Uuid::parse_str(&record.guest_role.id).unwrap(),
            ),
            created: record.created.into(),
            updated: match record.updated {
                None => None,
                Some(res) => Some(DateTime::from(res)),
            },
            accounts: None,
        },
        //
        // If not response were find, try to create a new record.
        //
        None => match client
            .guest_user()
            .create(
                guest_user.email.get_email(),
                guest_role_model::id::equals(
                    match guest_user.guest_role.to_owned() {
                        Parent::Id(id) => id.to_string(),
                        Parent::Record(record) => match record.id {
                            None => {
                                return creation_err(format!(
                                    "Role ID not available: {:?}",
                                    guest_user.id.to_owned(),
                                ))
                                .as_error()
                            }
                            Some(id) => id.to_string(),
                        },
                    },
                ),
                vec![],
            )
            .include(guest_user_model::include!({
                guest_role: select { id }
            }))
            .exec()
            .await
        {
            // !
            // ! Error case return
            // !
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    err
                ))
                .as_error();
            }
            Ok(record) => GuestUser {
                id: Some(Uuid::from_str(&record.id).unwrap()),
                email: Email::from_string(record.email.to_owned())?,
                guest_role: Parent::Id(
                    Uuid::parse_str(&record.guest_role.id).unwrap(),
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

    debug!("_guest_user (2): {:?}", _guest_user);

    // ? -----------------------------------------------------------------------
    // ? Create guest_user case not exists
    //
    // If the previous step returned a None response, try to create these
    // guest-user combination.
    //
    // ? -----------------------------------------------------------------------

    match client
        .guest_user_on_account()
        .create(
            guest_user_model::id::equals(match _guest_user.id {
                None => {
                    return creation_err(format!(
                        "Unexpected error on try to guest user: {:?}",
                        guest_user.id.to_owned(),
                    ))
                    .as_error()
                }
                Some(id) => id.to_string(),
            }),
            account_model::id::equals(account_id.to_string()),
            vec![],
        )
        .exec()
        .await
    {
        Err(err) => {
            if err.is_prisma_error::<UniqueKeyViolation>() {
                warn!("Guest user already exists: {err}");

                return creation_err("Guest user already exists")
                    .with_code(NativeErrorCodes::MYC00017)
                    .with_exp_true()
                    .as_error();
            };

            creation_err(format!("Unexpected error on create guest: {err}"))
                .as_error()
        }
        Ok(_) => Ok(GetOrCreateResponseKind::Created(_guest_user)),
    }
}
