use crate::{
    prisma::{account as account_model, user as user_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use myc_core::domain::{
    dtos::{
        account::VerboseStatus,
        account_type::AccountType,
        email::Email,
        native_error_codes::NativeErrorCodes,
        profile::{Owner, Profile},
    },
    entities::ProfileFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::process::id as process_id;
use tracing::error;
use uuid::Uuid;

#[derive(Component, Debug)]
#[shaku(interface = ProfileFetching)]
pub struct ProfileFetchingSqlDbRepository {}

#[async_trait]
impl ProfileFetching for ProfileFetchingSqlDbRepository {
    async fn get(
        &self,
        email: Option<Email>,
        _: Option<String>,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        let email = if let None = email {
            return fetching_err(String::from(
                "Email could not be empty during profile checking.",
            ))
            .as_error();
        } else {
            email.unwrap()
        };

        // ? -------------------------------------------------------------------
        // ? Build and execute the database query
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        let query = client
            .account()
            .find_first(vec![account_model::owners::some(vec![
                user_model::email::equals(email.email()),
            ])])
            .include(account_model::include!({
                owners: select {
                    id
                    email
                    first_name
                    last_name
                    username
                    is_active
                    is_principal
                }
            }));

        let response = query.exec().await.unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match response {
            Some(record) => {
                let account_type: AccountType =
                    match from_value(record.account_type) {
                        Ok(res) => res,
                        Err(err) => {
                            error!("Error on discovery account type: {err}");

                            return fetching_err(String::from(
                                "Unexpected error on discovery account type.",
                            ))
                            .as_error();
                        }
                    };

                let (is_subscription, is_manager, is_staff) = match account_type
                {
                    AccountType::Subscription { .. }
                    | AccountType::RoleAssociated { .. } => {
                        (true, false, false)
                    }
                    AccountType::Manager => (false, true, false),
                    AccountType::Staff => (false, true, true),
                    _ => (false, false, false),
                };

                Ok(FetchResponseKind::Found(Profile::new(
                    record
                        .owners
                        .iter()
                        .map(|owner| Owner {
                            id: Uuid::parse_str(&owner.id).unwrap(),
                            email: Email::from_string(owner.email.to_owned())
                                .unwrap()
                                .email(),
                            first_name: Some(owner.first_name.to_owned()),
                            last_name: Some(owner.last_name.to_owned()),
                            username: Some(owner.username.to_owned()),
                            is_principal: owner.is_principal,
                        })
                        .collect::<Vec<Owner>>(),
                    Uuid::parse_str(&record.id).unwrap(),
                    is_subscription,
                    is_manager,
                    is_staff,
                    record.owners.iter().any(|i| i.is_active == true),
                    record.is_active,
                    record.is_checked,
                    record.is_archived,
                    Some(VerboseStatus::from_flags(
                        record.is_active,
                        record.is_checked,
                        record.is_archived,
                    )),
                    None,
                )))
            }
            None => Ok(FetchResponseKind::NotFound(Some(email.email()))),
        }
    }
}
