use crate::{
    prisma::{account as account_model, user as user_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use myc_core::domain::{
    dtos::{
        email::EmailDTO,
        guest::PermissionsType,
        profile::{LicensedResourcesDTO, ProfileDTO},
    },
    entities::profile_fetching::ProfileFetching,
};
use shaku::Component;
use std::process::id as process_id;
use uuid::Uuid;

#[derive(Component, Debug)]
#[shaku(interface = ProfileFetching)]
pub struct ProfileFetchingSqlDbRepository {}

#[async_trait]
impl ProfileFetching for ProfileFetchingSqlDbRepository {
    async fn get(
        &self,
        email: EmailDTO,
    ) -> Result<FetchResponseKind<ProfileDTO, EmailDTO>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Build and execute the database query
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return Err(fetching_err(
                    String::from(
                        "Prisma Client error. Could not fetch client.",
                    ),
                    Some(false),
                    None,
                ))
            }
            Some(res) => res,
        };

        let query = client
            .account()
            .find_first(vec![account_model::owner::is(vec![
                user_model::email::equals(email.get_email()),
            ])])
            .include(account_model::include!({
                owner: select {
                    email
                }
                account_type: select {
                    is_subscription
                    is_manager
                    is_staff
                }
                guest_users: select {
                    guest_user: select {
                        created
                        updated
                        role: select {
                            permissions
                            role: select {
                                name
                            }
                        }
                    }
                    account_id
                }
            }));

        let response = query.exec().await.unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match response {
            Some(record) => {
                let record = record;
                let guests = record
                    .guest_users
                    .into_iter()
                    .map(|guest| LicensedResourcesDTO {
                        guest_account_id: Uuid::parse_str(
                            &guest.account_id.as_str(),
                        )
                        .unwrap(),
                        role: guest.guest_user.role.role.name,
                        permissions: guest
                            .guest_user
                            .role
                            .permissions
                            .into_iter()
                            .map(|i| PermissionsType::from_i32(i))
                            .collect(),
                        created: guest.guest_user.created.into(),
                        updated: match guest.guest_user.updated {
                            None => None,
                            Some(res) => Some(DateTime::from(res)),
                        },
                    })
                    .collect::<Vec<LicensedResourcesDTO>>();

                Ok(FetchResponseKind::Found(ProfileDTO {
                    email: match EmailDTO::from_string(record.owner.email) {
                        Err(err) => return Err(err),
                        Ok(res) => res.get_email(),
                    },
                    current_account_id: Uuid::parse_str(&record.id).unwrap(),
                    is_subscription: record.account_type.is_subscription,
                    is_manager: record.account_type.is_manager,
                    is_staff: record.account_type.is_staff,
                    licensed_resources: match guests.len() {
                        0 => None,
                        _ => Some(guests),
                    },
                }))
            }
            None => Ok(FetchResponseKind::NotFound(Some(email))),
        }
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use log::{error, warn};

    #[tokio::test]
    async fn test_if_fetching_works() {
        env_logger::init();
        let repo = ProfileFetchingSqlDbRepository {};

        warn!("repo: {:?}", repo);

        match repo
            .get(
                EmailDTO::from_string("username@domain.com".to_string())
                    .unwrap(),
            )
            .await
        {
            Err(err) => error!("err: {:?}", err),
            Ok(res) => warn!("res: {:?}", res),
        };
    }
}
