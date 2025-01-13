use crate::{models::config::DbConfig, schema::user as user_model};

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
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl UserDeletion for UserDeletionSqlDbRepository {
    async fn delete(
        &self,
        user_id: Uuid,
    ) -> Result<DeletionResponseKind<Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        match diesel::delete(user_model::table.find(user_id)).execute(conn) {
            Ok(_) => Ok(DeletionResponseKind::Deleted),
            Err(e) => {
                Ok(DeletionResponseKind::NotDeleted(user_id, e.to_string()))
            }
        }
    }
}
