use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use diesel::RunQueryDsl;
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
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TokenDeletion for TokenDeletionSqlDbRepository {
    #[tracing::instrument(
        name = "revoke_connection_string",
        skip_all
    )]
    async fn revoke_connection_string(
        &self,
        account_id: Uuid,
        token_id: u32,
    ) -> Result<DeletionResponseKind<u32>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let sql = format!(
            r#"
            UPDATE token
            SET expiration = NOW()
            WHERE id = {token_id}
              AND meta->>'accountId' = '{account_id}'
              AND meta ? 'token'
              AND meta ? 'name'
              AND meta ? 'id'
            "#,
            token_id = token_id,
            account_id = account_id,
        );

        let affected = diesel::sql_query(sql)
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

    #[tracing::instrument(
        name = "delete_connection_string",
        skip_all
    )]
    async fn delete_connection_string(
        &self,
        account_id: Uuid,
        token_id: u32,
    ) -> Result<DeletionResponseKind<u32>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            deletion_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let sql = format!(
            r#"
            DELETE FROM token
            WHERE id = {token_id}
              AND meta->>'accountId' = '{account_id}'
              AND meta ? 'token'
              AND meta ? 'name'
              AND meta ? 'id'
            "#,
            token_id = token_id,
            account_id = account_id,
        );

        let affected = diesel::sql_query(sql)
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
