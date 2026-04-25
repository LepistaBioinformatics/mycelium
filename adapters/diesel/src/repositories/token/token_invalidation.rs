use crate::{
    models::{config::DbPoolProvider, token::Token as TokenModel},
    schema::token as token_model,
};

use async_trait::async_trait;
use diesel::{Connection, QueryDsl, RunQueryDsl};
use myc_core::domain::{
    dtos::{
        email::Email,
        native_error_codes::NativeErrorCodes,
        token::{
            EmailConfirmationTokenMeta, MagicLinkTokenMeta, UserRelatedMeta,
        },
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
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl TokenInvalidation for TokenInvalidationSqlDbRepository {
    #[tracing::instrument(
        name = "get_and_invalidate_email_confirmation_token",
        skip_all
    )]
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
                WHERE meta->'email'->>'username' = '{username}' 
                AND meta->'email'->>'domain' = '{domain}' 
                AND meta->>'userId' = '{user_id}'
                "#,
                    username = meta.email.username,
                    domain = meta.email.domain,
                    user_id = meta.user_id
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

                if let Some(token) = tokens.first() {
                    if token.expiration < chrono::Utc::now().naive_utc() {
                        return Ok((None, false));
                    }

                    let token_meta: UserRelatedMeta<String> =
                        from_value(token.meta.clone()).map_err(|_| {
                            diesel::result::Error::RollbackTransaction
                        })?;

                    if let Err(e) =
                        token_meta.check_token(&meta.get_token().as_bytes())
                    {
                        error!("Invalid token: {}", e);
                        return Ok((None, false));
                    };

                    // Delete token
                    let deleted =
                        diesel::delete(token_model::table.find(token.id))
                            .execute(conn)
                            .map_err(|_| {
                                diesel::result::Error::RollbackTransaction
                            })?;

                    if deleted > 0 {
                        let token_meta: EmailConfirmationTokenMeta =
                            from_value(token.meta.clone()).map_err(|_| {
                                diesel::result::Error::RollbackTransaction
                            })?;
                        Ok((Some(token_meta.user_id), true))
                    } else {
                        Ok((None, false))
                    }
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

    #[tracing::instrument(
        name = "get_and_invalidate_password_change_token",
        skip_all
    )]
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
                SELECT id, expiration, meta 
                FROM token 
                WHERE meta->'email'->>'username' = '{username}' 
                AND meta->'email'->>'domain' = '{domain}' 
                AND meta->>'userId' = '{user_id}'
                "#,
                    username = meta.email.username,
                    domain = meta.email.domain,
                    user_id = meta.user_id
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

                if let Some(token) = tokens.first() {
                    if token.expiration < chrono::Utc::now().naive_utc() {
                        return Ok((None, false));
                    }

                    let token_meta: UserRelatedMeta<String> =
                        from_value(token.meta.clone()).map_err(|_| {
                            diesel::result::Error::RollbackTransaction
                        })?;

                    if let Err(e) =
                        token_meta.check_token(&meta.get_token().as_bytes())
                    {
                        error!("Invalid token: {}", e);
                        return Ok((None, false));
                    };

                    // Delete token
                    let deleted =
                        diesel::delete(token_model::table.find(token.id))
                            .execute(conn)
                            .map_err(|_| {
                                diesel::result::Error::RollbackTransaction
                            })?;

                    if deleted > 0 {
                        let token_meta: EmailConfirmationTokenMeta =
                            from_value(token.meta.clone()).map_err(|_| {
                                diesel::result::Error::RollbackTransaction
                            })?;
                        Ok((Some(token_meta.user_id), true))
                    } else {
                        Ok((None, false))
                    }
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

    #[tracing::instrument(
        name = "get_code_and_invalidate_display_token",
        skip_all
    )]
    async fn get_code_and_invalidate_display_token(
        &self,
        email: &Email,
        token: &str,
    ) -> Result<FetchResponseKind<String, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let username = email.username.replace('\'', "''");
        let domain = email.domain.replace('\'', "''");
        let token_val = token.replace('\'', "''");

        let result: Result<Option<(i32, String)>, diesel::result::Error> = conn
            .transaction(|conn| {
                // ? -----------------------------------------------------------
                // ? Fetch by (email, token) — token must not be null yet
                // ? -----------------------------------------------------------

                let sql = format!(
                    r#"
                    SELECT id, expiration, meta
                    FROM token
                    WHERE meta->>'token' = '{token}'
                    AND meta->'email'->>'username' = '{username}'
                    AND meta->'email'->>'domain' = '{domain}'
                    AND expiration > now()
                    LIMIT 1
                    "#,
                    token = token_val,
                    username = username,
                    domain = domain
                );

                let tokens = diesel::sql_query(sql)
                    .load::<TokenModel>(conn)
                    .map_err(|e| {
                        error!(
                            "Error fetching magic link display token: {}",
                            e
                        );
                        diesel::result::Error::RollbackTransaction
                    })?;

                let record = match tokens.into_iter().next() {
                    Some(r) => r,
                    None => return Ok(None),
                };

                let meta: MagicLinkTokenMeta = from_value(record.meta.clone())
                    .map_err(|e| {
                        error!("Error deserializing magic link meta: {}", e);
                        diesel::result::Error::RollbackTransaction
                    })?;

                let code = meta.code.clone();

                // ? -----------------------------------------------------------
                // ? Consume the display token — set token field to JSON null
                // ? -----------------------------------------------------------

                let update_sql = format!(
                    "UPDATE token \
                     SET meta = jsonb_set(meta, '{{token}}', 'null'::jsonb) \
                     WHERE id = {id}",
                    id = record.id
                );

                diesel::sql_query(update_sql).execute(conn).map_err(|e| {
                    error!("Error consuming magic link display token: {}", e);
                    diesel::result::Error::RollbackTransaction
                })?;

                Ok(Some((record.id, code)))
            });

        match result {
            Ok(Some((_, code))) => Ok(FetchResponseKind::Found(code)),
            Ok(None) => Ok(FetchResponseKind::NotFound(Some(
                "Token not found or already used".to_string(),
            ))),
            Err(e) => fetching_err(format!(
                "Unexpected error on fetching display token: {}",
                e
            ))
            .as_error(),
        }
    }

    #[tracing::instrument(
        name = "get_and_invalidate_magic_link_code",
        skip_all
    )]
    async fn get_and_invalidate_magic_link_code(
        &self,
        email: &Email,
        code: &str,
    ) -> Result<FetchResponseKind<(), String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let username = email.username.replace('\'', "''");
        let domain = email.domain.replace('\'', "''");
        let code_val = code.replace('\'', "''");

        let result: Result<bool, diesel::result::Error> =
            conn.transaction(|conn| {
                // ? -----------------------------------------------------------
                // ? Fetch by (email, code) where display token was consumed
                // ? -----------------------------------------------------------

                let sql = format!(
                    r#"
                    SELECT id, expiration, meta
                    FROM token
                    WHERE meta->>'code' = '{code}'
                    AND meta->'email'->>'username' = '{username}'
                    AND meta->'email'->>'domain' = '{domain}'
                    AND (meta->>'token') IS NULL
                    AND expiration > now()
                    LIMIT 1
                    "#,
                    code = code_val,
                    username = username,
                    domain = domain
                );

                let tokens = diesel::sql_query(sql)
                    .load::<TokenModel>(conn)
                    .map_err(|e| {
                        error!("Error fetching magic link code token: {}", e);
                        diesel::result::Error::RollbackTransaction
                    })?;

                let record = match tokens.into_iter().next() {
                    Some(r) => r,
                    None => return Ok(false),
                };

                // ? -----------------------------------------------------------
                // ? Consume — delete the record
                // ? -----------------------------------------------------------

                let deleted =
                    diesel::delete(token_model::table.find(record.id))
                        .execute(conn)
                        .map_err(|e| {
                            error!(
                                "Error deleting magic link code token: {}",
                                e
                            );
                            diesel::result::Error::RollbackTransaction
                        })?;

                Ok(deleted > 0)
            });

        match result {
            Ok(true) => Ok(FetchResponseKind::Found(())),
            Ok(false) => Ok(FetchResponseKind::NotFound(Some(
                "Code not found, already used, or display link not opened"
                    .to_string(),
            ))),
            Err(e) => fetching_err(format!(
                "Unexpected error on fetching magic link code: {}",
                e
            ))
            .as_error(),
        }
    }
}
