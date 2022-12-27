use crate::{
    prisma::{
        account as account_model, guest_role as guest_role_model,
        guest_user as guest_user_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{email::EmailDTO, guest::GuestUserDTO},
    entities::GuestUserRegistration,
};
use shaku::Component;
use std::{process::id as process_id, str::FromStr};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = GuestUserRegistration)]
pub struct GuestUserRegistrationSqlDbRepository {}

#[async_trait]
impl GuestUserRegistration for GuestUserRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        guest_user: GuestUserDTO,
        account_id: Uuid,
    ) -> Result<GetOrCreateResponseKind<GuestUserDTO>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(creation_err(
                    String::from(
                        "Prisma Client error. Could not fetch client.",
                    ),
                    Some(false),
                    None,
                ))
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Get or create the guest user
        // ? -------------------------------------------------------------------

        let _guest_user = match client
            .guest_user()
            .find_first(vec![
                guest_user_model::email::equals(
                    guest_user.email.to_owned().get_email(),
                ),
                guest_user_model::role_id::equals(
                    match guest_user.guest_role.to_owned() {
                        ParentEnum::Id(id) => id.to_string(),
                        ParentEnum::Record(record) => match record.id {
                            None => {
                                // !
                                // ! Error case return
                                // !
                                return Err(creation_err(
                                    String::from(
                                        "Unable to get the guest role ID.",
                                    ),
                                    Some(false),
                                    None,
                                ));
                            }
                            Some(id) => id.to_string(),
                        },
                    },
                ),
            ])
            .include(guest_user_model::include!({
                role: select { id }
            }))
            .exec()
            .await
        {
            // !
            // ! Error case return
            // !
            Err(err) => {
                return Err(creation_err(
                    format!("Unexpected error on check guest user: {:?}", err),
                    None,
                    None,
                ))
            }
            Ok(res) => match res {
                //
                // If the fetching operation find a object, try to parse the
                // response as a GuestUserDTO.
                //
                Some(record) => GuestUserDTO {
                    id: Some(Uuid::from_str(&record.id).unwrap()),
                    email: match EmailDTO::from_string(record.email) {
                        // !
                        // ! Error case return
                        // !
                        Err(err) => {
                            return Err(creation_err(
                                format!(
                                "Unexpected error on parse user email: {:?}",
                                err,
                            ),
                                None,
                                None,
                            ))
                        }
                        Ok(res) => res,
                    },
                    guest_role: ParentEnum::Id(
                        Uuid::parse_str(&record.role.id).unwrap(),
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
                                ParentEnum::Id(id) => id.to_string(),
                                ParentEnum::Record(record) => match record.id {
                                    None => {
                                        return Err(creation_err(
                                            format!(
                                                "Role ID not available: {:?}",
                                                guest_user.id.to_owned(),
                                            ),
                                            None,
                                            None,
                                        ))
                                    }
                                    Some(id) => id.to_string(),
                                },
                            },
                        ),
                        vec![],
                    )
                    .include(guest_user_model::include!({
                        role: select { id }
                    }))
                    .exec()
                    .await
                {
                    // !
                    // ! Error case return
                    // !
                    Err(err) => {
                        return Err(creation_err(
                            format!(
                                "Unexpected error detected on create record: {}",
                                err
                            ),
                            None,
                            None,
                        ));
                    }
                    Ok(record) => GuestUserDTO {
                        id: Some(Uuid::from_str(&record.id).unwrap()),
                        email: match EmailDTO::from_string(
                            record.email.to_owned(),
                        ) {
                            // !
                            // ! Error case return
                            // !
                            Err(err) => {
                                return Err(creation_err(
                                    format!(
                                "Unexpected error on parse user email: {:?}",
                                err,
                            ),
                                    None,
                                    None,
                                ))
                            }
                            Ok(res) => res,
                        },
                        guest_role: ParentEnum::Id(
                            Uuid::parse_str(&record.role.id).unwrap(),
                        ),
                        created: record.created.into(),
                        updated: match record.updated {
                            None => None,
                            Some(res) => Some(DateTime::from(res)),
                        },
                        accounts: None,
                    },
                },
            },
        };

        println!("_guest_user: {:?}", _guest_user);

        match client
            .guest_user_on_account()
            .create(
                guest_user_model::id::equals(match _guest_user.id {
                    None => {
                        return Err(creation_err(
                            format!(
                                "Unexpected error on try to guest user: {:?}",
                                guest_user.id.to_owned(),
                            ),
                            None,
                            None,
                        ))
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
                return Err(creation_err(
                    format!("Unexpected error on create guest: {:?}", err,),
                    None,
                    None,
                ))
            }
            Ok(res) => res,
        };

        self.get_or_create(guest_user, account_id).await
    }
}
