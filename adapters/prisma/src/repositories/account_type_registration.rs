use crate::{
    prisma::account_type as account_type_model,
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::GetOrCreateResponseKind,
    utils::errors::{factories::creation_err, MappedErrors},
};
use myc_core::domain::{
    dtos::account::AccountType, entities::AccountTypeRegistration,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = AccountTypeRegistration)]
pub struct AccountTypeRegistrationSqlDbRepository {}

#[async_trait]
impl AccountTypeRegistration for AccountTypeRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        account_type: AccountType,
    ) -> Result<GetOrCreateResponseKind<AccountType>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return creation_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code("MYC00001".to_string())
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let response = client
            .account_type()
            .find_first(vec![account_type_model::name::equals(
                account_type.name.to_owned(),
            )])
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                let record = record;
                return Ok(GetOrCreateResponseKind::NotCreated(
                    AccountType {
                        id: Some(Uuid::parse_str(&record.id).unwrap()),
                        name: record.name,
                        description: record.description,
                        is_subscription: record.is_subscription,
                        is_manager: record.is_manager,
                        is_staff: record.is_staff,
                    },
                    String::from("Account type already exists"),
                ));
            }
            None => (),
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        let response = client
            .account_type()
            .create(
                account_type.name.to_owned(),
                account_type.description.to_owned(),
                vec![
                    account_type_model::is_subscription::set(
                        account_type.is_subscription,
                    ),
                    account_type_model::is_manager::set(
                        account_type.is_manager,
                    ),
                    account_type_model::is_staff::set(account_type.is_staff),
                ],
            )
            .exec()
            .await;

        match response {
            Ok(record) => {
                let record = record;

                Ok(GetOrCreateResponseKind::Created(AccountType {
                    id: Some(Uuid::parse_str(&record.id).unwrap()),
                    name: record.name,
                    description: record.description,
                    is_subscription: record.is_subscription,
                    is_manager: record.is_manager,
                    is_staff: record.is_staff,
                }))
            }
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on create record: {}",
                    err
                ))
                .with_exp_false()
                .as_error();
            }
        }
    }
}
