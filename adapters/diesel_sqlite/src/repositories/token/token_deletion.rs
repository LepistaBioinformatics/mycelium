use crate::{config::SqliteDbPoolProvider, types::naive_timestamp_to_text};

use async_trait::async_trait;
use chrono::Utc;
use diesel::{sql_types::Text, RunQueryDsl};
use myc_core::domain::{
    dtos::native_error_codes::NativeErrorCodes, entities::TokenDeletion,
};
use mycelium_base::{
    entities::DeletionResponseKind,
    utils::errors::{deletion_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenDeletion)]
pub struct TokenDeletionSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TokenDeletion for TokenDeletionSqlDbRepository {
    #[tracing::instrument(name = "revoke_connection_string", skip_all)]
    async fn revoke_connection_string(
        &self,
        account_id: Uuid,
        token_id: u32,
    ) -> Result<DeletionResponseKind<u32>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let sql = r#"
            UPDATE token
            SET expiration = ?
            WHERE id = ?
              AND json_extract(meta, '$.accountId') = ?
              AND json_extract(meta, '$.token') IS NOT NULL
              AND json_extract(meta, '$.name') IS NOT NULL
              AND json_extract(meta, '$.id') IS NOT NULL
        "#;

        let affected = diesel::sql_query(sql)
            .bind::<Text, _>(naive_timestamp_to_text(&Utc::now().naive_utc()))
            .bind::<diesel::sql_types::Integer, _>(token_id as i32)
            .bind::<Text, _>(account_id.to_string())
            .execute(conn)
            .map_err(|e| {
                error!("Error revoking connection string: {}", e);
                deletion_err(format!(
                    "Failed to revoke connection string: {}",
                    e
                ))
            })?;

        if affected == 0 {
            return Ok(DeletionResponseKind::NotDeleted(
                token_id,
                "Token not found or not owned by account".to_string(),
            ));
        }

        Ok(DeletionResponseKind::Deleted)
    }

    #[tracing::instrument(name = "delete_connection_string", skip_all)]
    async fn delete_connection_string(
        &self,
        account_id: Uuid,
        token_id: u32,
    ) -> Result<DeletionResponseKind<u32>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let sql = r#"
            DELETE FROM token
            WHERE id = ?
              AND json_extract(meta, '$.accountId') = ?
              AND json_extract(meta, '$.token') IS NOT NULL
              AND json_extract(meta, '$.name') IS NOT NULL
              AND json_extract(meta, '$.id') IS NOT NULL
        "#;

        let affected = diesel::sql_query(sql)
            .bind::<diesel::sql_types::Integer, _>(token_id as i32)
            .bind::<Text, _>(account_id.to_string())
            .execute(conn)
            .map_err(|e| {
                error!("Error deleting connection string: {}", e);
                deletion_err(format!(
                    "Failed to delete connection string: {}",
                    e
                ))
            })?;

        if affected == 0 {
            return Ok(DeletionResponseKind::NotDeleted(
                token_id,
                "Token not found or not owned by account".to_string(),
            ));
        }

        Ok(DeletionResponseKind::Deleted)
    }
}
