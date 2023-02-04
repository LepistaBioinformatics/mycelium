use super::functions::get_today_key;
use crate::repositories::connector::get_connection;

use async_trait::async_trait;
use clean_base::{
    entities::default_response::DeletionManyResponseKind,
    utils::errors::{creation_err, MappedErrors},
};
use myc_core::domain::entities::TokenCleanup;
use shaku::Component;

#[derive(Component)]
#[shaku(interface = TokenCleanup)]
pub struct TokenCleanupMemDbRepository {}

#[async_trait]
impl TokenCleanup for TokenCleanupMemDbRepository {
    async fn clean(
        &self,
        ignore_today: Option<bool>,
    ) -> Result<DeletionManyResponseKind<Vec<String>>, MappedErrors> {
        let ignore_today = ignore_today.unwrap_or(false);

        // ? -------------------------------------------------------------------
        // ? Try to build connection
        // ? -------------------------------------------------------------------

        let mut conn = get_connection().await;

        // ? -------------------------------------------------------------------
        // ? Try to persist data
        // ? -------------------------------------------------------------------

        let records_to_delete =
            match redis::cmd("KEYS").arg("*").query::<Vec<String>>(&mut conn) {
                Err(err) => {
                    return Err(creation_err(
                        format!(
                        "Unexpected error detected on retrieve record: {err}"
                    ),
                        None,
                        None,
                    ));
                }
                Ok(res) => {
                    if !ignore_today {
                        res
                    } else {
                        res.to_owned()
                            .into_iter()
                            .filter(|i| !i.eq(&get_today_key()))
                            .collect::<Vec<String>>()
                    }
                }
            };

        match redis::cmd("DEL")
            .arg(records_to_delete.join(" "))
            .query::<i64>(&mut conn)
        {
            Err(err) => {
                return Err(creation_err(
                    format!(
                        "Unexpected error detected on retrieve record: {err}"
                    ),
                    None,
                    None,
                ));
            }
            Ok(res) => return Ok(DeletionManyResponseKind::Deleted(res)),
        }
    }
}

// * ---------------------------------------------------------------------------
// * TESTS
// * ---------------------------------------------------------------------------

#[cfg(test)]
mod test {
    use super::*;
    use log::{error, info};
    use test_log::test;

    #[test(tokio::test)]
    async fn test_token_cleanup_works() {
        let repo = TokenCleanupMemDbRepository {};

        match repo.clean(None).await {
            Err(err) => error!("test err: {err}"),
            Ok(res) => info!("test res: {:?}", res),
        };
    }
}
