use crate::{
    prisma::{account as account_model, user as user_model},
    repositories::connector::get_client,
};

use async_trait::async_trait;
use clean_base::{
    entities::default_response::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use log::debug;
use myc_core::domain::{
    dtos::{email::Email, profile::Profile},
    entities::ProfileFetching,
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
        email: Option<Email>,
        _: Option<String>,
    ) -> Result<FetchResponseKind<Profile, String>, MappedErrors> {
        let email = if let None = email {
            return Err(fetching_err(
                String::from(
                    "Email could not be empty during profile checking.",
                ),
                Some(false),
                None,
            ));
        } else {
            email.unwrap()
        };

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
                    is_active
                }
                account_type: select {
                    is_subscription
                    is_manager
                    is_staff
                }
            }));

        let response = query.exec().await.unwrap();

        // ? -------------------------------------------------------------------
        // ? Evaluate and parse the database response
        // ? -------------------------------------------------------------------

        match response {
            Some(record) => {
                debug!("Profile record: {:?}", record);

                Ok(FetchResponseKind::Found(Profile {
                    email: match Email::from_string(record.owner.email) {
                        Err(err) => return Err(err),
                        Ok(res) => res.get_email(),
                    },
                    current_account_id: Uuid::parse_str(&record.id).unwrap(),
                    is_subscription: record.account_type.is_subscription,
                    is_manager: record.account_type.is_manager,
                    is_staff: record.account_type.is_staff,
                    owner_is_active: record.owner.is_active,
                    account_is_active: record.is_active,
                    account_was_approved: record.is_checked,
                    account_was_archived: record.is_archived,
                    licensed_resources: None,
                }))
            }
            None => Ok(FetchResponseKind::NotFound(Some(email.get_email()))),
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
                Some(
                    Email::from_string("username@domain.com".to_string())
                        .unwrap(),
                ),
                None,
            )
            .await
        {
            Err(err) => error!("err: {:?}", err),
            Ok(res) => warn!("res: {:?}", res),
        };
    }
}
