use crate::{
    models::{config::DbConfig, token::Token as TokenModel},
    schema::token as token_model,
};

use async_trait::async_trait;
use diesel::{Connection, QueryDsl, RunQueryDsl};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes, token::EmailConfirmationTokenMeta,
    },
    entities::TokenInvalidation,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenInvalidation)]
pub struct TokenInvalidationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbConfig>,
}

#[async_trait]
impl TokenInvalidation for TokenInvalidationSqlDbRepository {
    async fn get_and_invalidate_email_confirmation_token(
        &self,
        meta: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result: Result<(Option<Uuid>, bool), diesel::result::Error> = conn
            .transaction(|conn| {
                let sql = format!(
                    r#"
                SELECT id, expiration, meta 
                FROM token 
                WHERE meta->'email'->>'username' = '{}' 
                AND meta->'email'->>'domain' = '{}' 
                AND meta->>'userId' = '{}'
                "#,
                    meta.email.username, meta.email.domain, meta.user_id
                );

                let tokens = diesel::sql_query(sql)
                    .load::<TokenModel>(conn)
                    .map_err(|e| {
                        error!("Error fetching token: {}", e);

                        diesel::result::Error::RollbackTransaction
                    })?;

                if tokens.is_empty() {
                    return Ok((None, false));
                }

                // Get token with earliest expiration
                let mut tokens = tokens;
                tokens.sort_by(|a, b| a.expiration.cmp(&b.expiration));

                let token = &tokens[0];
                if token.expiration < chrono::Utc::now().naive_utc() {
                    return Ok((None, false));
                }

                // Delete token
                let deleted = diesel::delete(token_model::table.find(token.id))
                    .execute(conn)
                    .map_err(|_| diesel::result::Error::RollbackTransaction)?;

                if deleted > 0 {
                    let token_meta: EmailConfirmationTokenMeta =
                        from_value(token.meta.clone()).map_err(|_| {
                            diesel::result::Error::RollbackTransaction
                        })?;
                    Ok((Some(token_meta.user_id), true))
                } else {
                    Ok((None, false))
                }
            });

        match result {
            Ok((Some(user_id), true)) => Ok(FetchResponseKind::Found(user_id)),
            Ok((None, false)) => Ok(FetchResponseKind::NotFound(Some(
                "Invalid token".to_string(),
            ))),
            Ok(_) => Ok(FetchResponseKind::NotFound(Some(
                "Invalid operation".to_string(),
            ))),
            Err(e) => fetching_err(format!(
                "Unexpected error detected on fetching token: {}",
                e
            ))
            .as_error(),
        }
    }

    async fn get_and_invalidate_password_change_token(
        &self,
        meta: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let result: Result<(Option<Uuid>, bool), diesel::result::Error> = conn
            .transaction(|conn| {
                let sql = format!(
                    r#"
                SELECT DISTINCT id, expiration, meta 
                FROM token 
                WHERE meta->'email'->>'username' = '{}' 
                AND meta->'email'->>'domain' = '{}' 
                AND meta->>'userId' = '{}'
                ORDER BY expiration DESC 
                LIMIT 1
                "#,
                    meta.email.username, meta.email.domain, meta.user_id
                );

                let tokens = diesel::sql_query(sql)
                    .load::<TokenModel>(conn)
                    .map_err(|e| {
                        error!("Error fetching token: {}", e);

                        diesel::result::Error::RollbackTransaction
                    })?;

                if tokens.is_empty() {
                    return Ok((None, false));
                }

                let token = &tokens[0];
                if token.expiration < chrono::Utc::now().naive_utc() {
                    return Ok((None, false));
                }

                // Delete token
                let deleted = diesel::delete(token_model::table.find(token.id))
                    .execute(conn)
                    .map_err(|_| diesel::result::Error::RollbackTransaction)?;

                if deleted > 0 {
                    let token_meta: EmailConfirmationTokenMeta =
                        from_value(token.meta.clone()).map_err(|_| {
                            diesel::result::Error::RollbackTransaction
                        })?;
                    Ok((Some(token_meta.user_id), true))
                } else {
                    Ok((None, false))
                }
            });

        match result {
            Ok((Some(user_id), true)) => Ok(FetchResponseKind::Found(user_id)),
            Ok((None, false)) => Ok(FetchResponseKind::NotFound(Some(
                "Invalid token".to_string(),
            ))),
            Ok(_) => Ok(FetchResponseKind::NotFound(Some(
                "Invalid operation".to_string(),
            ))),
            Err(e) => fetching_err(format!(
                "Unexpected error detected on fetching token: {}",
                e
            ))
            .as_error(),
        }
    }
}
