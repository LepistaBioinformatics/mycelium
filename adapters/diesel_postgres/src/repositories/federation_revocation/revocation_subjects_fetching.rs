use crate::{
    models::config::DbPoolProvider,
    schema::{
        guest_user as guest_model, guest_user_on_account as guest_on_model,
        user as user_model,
    },
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::entities::RevocationSubjectsFetching;
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = RevocationSubjectsFetching)]
pub struct RevocationSubjectsFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl RevocationSubjectsFetchingSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

fn read_err(e: diesel::result::Error) -> MappedErrors {
    fetching_err(format!("Failed to read revocation subjects: {e}"))
}

#[async_trait]
impl RevocationSubjectsFetching for RevocationSubjectsFetchingSqlDbRepository {
    #[tracing::instrument(name = "revocation_emails_of_account", skip_all)]
    async fn emails_of_account(
        &self,
        account_id: Uuid,
    ) -> Result<Vec<String>, MappedErrors> {
        let conn = &mut *self.connection()?;
        let mut emails = user_model::table
            .filter(user_model::account_id.eq(account_id))
            .select(user_model::email)
            .load::<String>(conn)
            .map_err(read_err)?;

        emails.extend(
            guest_on_model::table
                .inner_join(guest_model::table)
                .filter(guest_on_model::account_id.eq(account_id))
                .select(guest_model::email)
                .load::<String>(conn)
                .map_err(read_err)?,
        );
        emails.sort();
        emails.dedup();

        Ok(emails)
    }

    #[tracing::instrument(name = "revocation_emails_of_guest_role", skip_all)]
    async fn emails_of_guest_role(
        &self,
        guest_role_id: Uuid,
    ) -> Result<Vec<String>, MappedErrors> {
        guest_model::table
            .filter(guest_model::guest_role_id.eq(guest_role_id))
            .select(guest_model::email)
            .distinct()
            .load::<String>(&mut *self.connection()?)
            .map_err(read_err)
    }
}
