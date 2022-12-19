use crate::{
    adapters::repositories::sql_db::connector::get_client,
    domain::{
        dtos::{
            email::EmailDTO,
            guest::PermissionsType,
            profile::{LicensedIdentifiersDTO, ProfileDTO},
        },
        entities::service::profile_fetching::ProfileFetching,
    },
};

use async_trait::async_trait;
use chrono::DateTime;
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use myc_prisma::prisma::{account as account_model, user as user_model};
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
                        email
                        created
                        updated
                        role_id
                        role: select {
                            permissions
                        }
                    }
                    account_id
                    created
                }
            }));

        let response = query.exec().await.unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match response {
            Some(record) => {
                let record = record;
                let id = Uuid::parse_str(&record.id).unwrap();

                let email = match EmailDTO::from_string(record.owner.email) {
                    Err(err) => return Err(err),
                    Ok(res) => res,
                };

                let guests = record
                    .guest_users
                    .into_iter()
                    .map(|guest| {
                        let perms = guest
                            .guest_user
                            .role
                            .permissions
                            .into_iter()
                            .map(|i| PermissionsType::from_i32(i))
                            .collect();

                        let updated = match guest.guest_user.updated {
                            None => None,
                            Some(res) => Some(DateTime::from(res)),
                        };

                        LicensedIdentifiersDTO {
                            account_id: Uuid::parse_str(
                                &guest.account_id.as_str(),
                            )
                            .unwrap(),
                            role_id: Uuid::parse_str(
                                &guest.guest_user.role_id.as_str(),
                            )
                            .unwrap(),
                            permissions: perms,
                            created: guest.guest_user.created.into(),
                            updated,
                        }
                    })
                    .collect::<Vec<LicensedIdentifiersDTO>>();

                Ok(FetchResponseKind::Found(ProfileDTO {
                    email,
                    account_id: id,
                    is_subscription: record.account_type.is_subscription,
                    is_manager: record.account_type.is_manager,
                    is_staff: record.account_type.is_staff,
                    licensed_ids: match guests.len() {
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
