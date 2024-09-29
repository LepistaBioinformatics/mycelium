use crate::{
    prisma::{
        identity_provider as identity_provider_model, user as user_model,
    },
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::Local;
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        user::{Provider, User},
    },
    entities::UserRegistration,
};
use mycelium_base::{
    dtos::Parent,
    entities::{CreateResponseKind, GetOrCreateResponseKind},
    utils::errors::{creation_err, MappedErrors},
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserRegistration)]
pub struct UserRegistrationSqlDbRepository {}

#[async_trait]
impl UserRegistration for UserRegistrationSqlDbRepository {
    async fn get_or_create(
        &self,
        user: User,
    ) -> Result<GetOrCreateResponseKind<User>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Perform basic validations
        // ? -------------------------------------------------------------------

        let provider = match user.to_owned().provider() {
            None => {
                return creation_err(String::from(
                    "Provider is required to create a user",
                ))
                .with_code(NativeErrorCodes::MYC00002)
                .as_error()
            }
            Some(provider) => provider,
        };

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

        // ? -------------------------------------------------------------------
        // ? Build the initial query (get part of the get-or-create)
        // ? -------------------------------------------------------------------

        let response = client
            .user()
            .find_first(vec![user_model::email::equals(user.email.get_email())])
            .exec()
            .await;

        match response.unwrap() {
            Some(record) => {
                return Ok(GetOrCreateResponseKind::NotCreated(
                    User::new(
                        Some(Uuid::parse_str(&record.id).unwrap()),
                        record.username,
                        Email::from_string(record.email)?,
                        Some(record.first_name),
                        Some(record.last_name),
                        record.is_active,
                        record.created.into(),
                        match record.updated {
                            None => None,
                            Some(date) => Some(date.with_timezone(&Local)),
                        },
                        match &record.account_id {
                            Some(id) => {
                                Some(Parent::Id(Uuid::parse_str(id).unwrap()))
                            }
                            None => None,
                        },
                        None,
                    )
                    .with_principal(record.is_principal),
                    "User already exists".to_string(),
                ));
            }
            None => (),
        };

        // ? -------------------------------------------------------------------
        // ? Build create part of the get-or-create
        // ? -------------------------------------------------------------------

        let response = client
            ._transaction()
            .run(|client| async move {
                let user_instance = client
                    .user()
                    .create(
                        user.to_owned().username,
                        user.to_owned().email.get_email(),
                        user.to_owned().first_name.unwrap_or(String::from("")),
                        user.to_owned().last_name.unwrap_or(String::from("")),
                        vec![
                            user_model::is_active::set(user.is_active),
                            user_model::is_principal::set(user.is_principal()),
                        ],
                    )
                    //.include(user_model::include!({ account }))
                    .exec()
                    .await?;

                let mut provider_params = vec![];

                if let Provider::External(name) = provider {
                    provider_params
                        .push(identity_provider_model::name::set(Some(name)));
                } else if let Provider::Internal(pass) = provider {
                    provider_params.push(
                        identity_provider_model::password_hash::set(Some(
                            pass.to_owned().hash,
                        )),
                    );
                };

                client
                    .identity_provider()
                    .create(
                        user_model::id::equals(user_instance.id.to_string()),
                        provider_params,
                    )
                    .exec()
                    .await
                    .map(|_| user_instance)
            })
            .await;

        match response {
            Ok(record) => Ok(GetOrCreateResponseKind::Created(
                User::new(
                    Some(Uuid::parse_str(&record.id).unwrap()),
                    record.username,
                    Email::from_string(record.email)?,
                    Some(record.first_name),
                    Some(record.last_name),
                    record.is_active,
                    record.created.into(),
                    match record.updated {
                        None => None,
                        Some(date) => Some(date.with_timezone(&Local)),
                    },
                    match record.account_id {
                        None => None,
                        Some(id) => {
                            Some(Parent::Id(Uuid::parse_str(&id).unwrap()))
                        }
                    },
                    None,
                )
                .with_principal(record.is_principal),
            )),
            Err(err) => {
                return creation_err(format!(
                    "Unexpected error detected on update record: {}",
                    err
                ))
                .as_error();
            }
        }
    }

    // ? -----------------------------------------------------------------------
    // ! NOT IMPLEMENTED METHODS
    // ? -----------------------------------------------------------------------

    async fn create(
        &self,
        user: User,
    ) -> Result<CreateResponseKind<User>, MappedErrors> {
        self.create(user).await
    }
}
