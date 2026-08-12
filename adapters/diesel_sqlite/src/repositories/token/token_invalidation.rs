use crate::{
    config::SqliteDbPoolProvider, models::token::Token as TokenModel,
    schema::token, types::naive_timestamp_to_text,
};

use async_trait::async_trait;
use chrono::Utc;
use diesel::{sql_types::Text, Connection, QueryDsl, RunQueryDsl};
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
use shaku::Component;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenInvalidation)]
pub struct TokenInvalidationSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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
        self.get_and_invalidate_user_related_token(meta)
    }

    #[tracing::instrument(
        name = "get_and_invalidate_password_change_token",
        skip_all
    )]
    async fn get_and_invalidate_password_change_token(
        &self,
        meta: EmailConfirmationTokenMeta,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        // Identical lookup/consume flow to the email-confirmation variant
        // (both operate on `UserRelatedMeta<String>`); mirrors the postgres
        // repo, which also implements both methods with the same body.
        self.get_and_invalidate_user_related_token(meta)
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

        let username = email.username.clone();
        let domain = email.domain.clone();
        let token_val = token.to_string();
        let now = naive_timestamp_to_text(&Utc::now().naive_utc());

        let result: Result<Option<(i32, String)>, diesel::result::Error> = conn
            .transaction(|conn| {
                // ? -----------------------------------------------------------
                // ? Fetch by (email, token) — token must not be null yet
                // ? -----------------------------------------------------------

                let sql = r#"
                    SELECT id, expiration, meta
                    FROM token
                    WHERE json_extract(meta, '$.token') = ?
                    AND json_extract(meta, '$.email.username') = ?
                    AND json_extract(meta, '$.email.domain') = ?
                    AND expiration > ?
                    LIMIT 1
                "#;

                let tokens = diesel::sql_query(sql)
                    .bind::<Text, _>(&token_val)
                    .bind::<Text, _>(&username)
                    .bind::<Text, _>(&domain)
                    .bind::<Text, _>(&now)
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

                let meta: MagicLinkTokenMeta =
                    serde_json::from_str(&record.meta).map_err(|e| {
                        error!("Error deserializing magic link meta: {}", e);
                        diesel::result::Error::RollbackTransaction
                    })?;

                let code = meta.code.clone();

                // ? -----------------------------------------------------------
                // ? Consume the display token — set token field to JSON null
                // ? -----------------------------------------------------------

                diesel::sql_query(
                    "UPDATE token SET meta = json_set(meta, '$.token', json('null')) WHERE id = ?",
                )
                .bind::<diesel::sql_types::Integer, _>(record.id)
                .execute(conn)
                .map_err(|e| {
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

        let username = email.username.clone();
        let domain = email.domain.clone();
        let code_val = code.to_string();
        let now = naive_timestamp_to_text(&Utc::now().naive_utc());

        let result: Result<bool, diesel::result::Error> =
            conn.transaction(|conn| {
                // ? -----------------------------------------------------------
                // ? Fetch by (email, code) where display token was consumed
                // ? -----------------------------------------------------------

                let sql = r#"
                    SELECT id, expiration, meta
                    FROM token
                    WHERE json_extract(meta, '$.code') = ?
                    AND json_extract(meta, '$.email.username') = ?
                    AND json_extract(meta, '$.email.domain') = ?
                    AND json_extract(meta, '$.token') IS NULL
                    AND expiration > ?
                    LIMIT 1
                "#;

                let tokens = diesel::sql_query(sql)
                    .bind::<Text, _>(&code_val)
                    .bind::<Text, _>(&username)
                    .bind::<Text, _>(&domain)
                    .bind::<Text, _>(&now)
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

                let deleted = diesel::delete(token::table.find(record.id))
                    .execute(conn)
                    .map_err(|e| {
                        error!("Error deleting magic link code token: {}", e);
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

impl TokenInvalidationSqlDbRepository {
    fn get_and_invalidate_user_related_token(
        &self,
        meta: UserRelatedMeta<String>,
    ) -> Result<FetchResponseKind<Uuid, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let username = meta.email.username.clone();
        let domain = meta.email.domain.clone();
        let user_id = meta.user_id.to_string();

        let result: Result<(Option<Uuid>, bool), diesel::result::Error> = conn
            .transaction(|conn| {
                let sql = r#"
                    SELECT id, expiration, meta
                    FROM token
                    WHERE json_extract(meta, '$.email.username') = ?
                    AND json_extract(meta, '$.email.domain') = ?
                    AND json_extract(meta, '$.userId') = ?
                "#;

                let tokens = diesel::sql_query(sql)
                    .bind::<Text, _>(&username)
                    .bind::<Text, _>(&domain)
                    .bind::<Text, _>(&user_id)
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

                let Some(token) = tokens.first() else {
                    return Ok((None, false));
                };

                let now = naive_timestamp_to_text(&Utc::now().naive_utc());
                if token.expiration < now {
                    return Ok((None, false));
                }

                let token_meta: UserRelatedMeta<String> =
                    serde_json::from_str(&token.meta).map_err(|_| {
                        diesel::result::Error::RollbackTransaction
                    })?;

                if let Err(e) =
                    token_meta.check_token(meta.get_token().as_bytes())
                {
                    error!("Invalid token: {}", e);
                    return Ok((None, false));
                };

                // Delete token
                let deleted = diesel::delete(token::table.find(token.id))
                    .execute(conn)
                    .map_err(|_| diesel::result::Error::RollbackTransaction)?;

                if deleted == 0 {
                    return Ok((None, false));
                }

                let token_meta: EmailConfirmationTokenMeta =
                    serde_json::from_str(&token.meta).map_err(|_| {
                        diesel::result::Error::RollbackTransaction
                    })?;

                Ok((Some(token_meta.user_id), true))
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
