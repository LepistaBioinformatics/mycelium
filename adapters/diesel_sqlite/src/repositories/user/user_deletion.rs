use crate::{config::SqliteDbPoolProvider, schema::user, types::uuid_to_text};

use async_trait::async_trait;
use diesel::prelude::*;
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::UserDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = UserDeletion)]
pub struct UserDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl UserDeletion for UserDeletionSqlDbRepository {
    #[tracing::instrument(name = "delete_user", skip_all)]
    async fn delete(
        &self,
        user_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        match diesel::delete(user::table.find(uuid_to_text(&user_id)))
            .execute(conn)
        {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(e) => {
                Ok(DeletionResponseKind::NotDeleted(user_id, e.to_string()))
            }
        }
    }
}
