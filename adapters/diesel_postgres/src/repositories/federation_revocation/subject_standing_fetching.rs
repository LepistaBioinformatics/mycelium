use crate::{
    models::config::DbPoolProvider,
    schema::{account as account_model, user as user_model},
};

use async_trait::async_trait;
use diesel::{
    prelude::*,
    r2d2::{ConnectionManager, PooledConnection},
};
use myc_core::domain::{
    dtos::federation::SubjectStanding, entities::SubjectStandingFetching,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

/// The user's flag and, when the user has an account, the account's flags
type StandingRow = (bool, Option<(bool, bool, bool)>);

#[derive(Component)]
#[shaku(interface = SubjectStandingFetching)]
pub struct SubjectStandingFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

impl SubjectStandingFetchingSqlDbRepository {
    fn connection(
        &self,
    ) -> Result<PooledConnection<ConnectionManager<PgConnection>>, MappedErrors>
    {
        self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
        })
    }
}

/// A user without an account has no profile, so it stands as inactive
fn map_standing((owner_is_active, account): StandingRow) -> SubjectStanding {
    let (account_is_active, account_was_archived, account_was_deleted) =
        account.unwrap_or((false, false, false));

    SubjectStanding {
        owner_is_active,
        account_is_active,
        account_was_archived,
        account_was_deleted,
    }
}

#[async_trait]
impl SubjectStandingFetching for SubjectStandingFetchingSqlDbRepository {
    #[tracing::instrument(name = "subject_standing", skip_all)]
    async fn standing(
        &self,
        user_id: Uuid,
    ) -> Result<Option<SubjectStanding>, MappedErrors> {
        let row = user_model::table
            .left_join(account_model::table)
            .filter(user_model::id.eq(user_id))
            .select((
                user_model::is_active,
                (
                    account_model::is_active,
                    account_model::is_archived,
                    account_model::is_deleted,
                )
                    .nullable(),
            ))
            .first::<StandingRow>(&mut *self.connection()?)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to read the subject: {e}"))
            })?;

        Ok(row.map(map_standing))
    }
}
