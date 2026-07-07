use super::map_public_connection_string_info_model_to_dto;
use crate::sqlite::{
    config::SqliteDbPoolProvider,
    models::{
        public_connection_string_info::PublicConnectionStringInfoModel,
        token::Token as TokenModel,
    },
    types::naive_timestamp_from_text,
};

use async_trait::async_trait;
use chrono::Local;
use diesel::{sql_types::Text, RunQueryDsl};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{
            ConnectionStringBean, MultiTypeMeta, PublicConnectionStringInfo,
            Token, UserAccountConnectionString, UserAccountScope,
        },
    },
    entities::TokenFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::sync::Arc;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenFetching)]
pub struct TokenFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

#[async_trait]
impl TokenFetching for TokenFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_connection_string", skip_all)]
    async fn get_connection_string(
        &self,
        scope: UserAccountScope,
    ) -> Result<FetchResponseKind<Token, String>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let beans = scope.get_scope_beans();

        let account_id = match beans.iter().find_map(|bean| {
            if let &ConnectionStringBean::AID(account_id) = bean {
                Some(account_id)
            } else {
                None
            }
        }) {
            Some(id) => id,
            None => {
                return Ok(FetchResponseKind::NotFound(Some(
                    "Account ID not found".to_string(),
                )))
            }
        };

        let signature = match beans.iter().find_map(|bean| {
            if let ConnectionStringBean::SIG(signature) = bean {
                Some(signature)
            } else {
                None
            }
        }) {
            Some(sig) => sig.to_owned(),
            None => {
                return Ok(FetchResponseKind::NotFound(Some(
                    "Signature not found".to_string(),
                )))
            }
        };

        // Each `scope` entry is a compact JSON object like {"aid": "..."} or
        // {"sig": "..."}; json_each/json_extract walk the array the same way
        // postgres's jsonb_array_elements does.
        let sql = r#"
            SELECT id, expiration, meta
            FROM token
            WHERE EXISTS (
                SELECT 1 FROM json_each(token.meta, '$.scope') AS elem
                WHERE json_extract(elem.value, '$.aid') = ?
            )
            AND EXISTS (
                SELECT 1 FROM json_each(token.meta, '$.scope') AS elem
                WHERE json_extract(elem.value, '$.sig') = ?
            )
        "#;

        let tokens = diesel::sql_query(sql)
            .bind::<Text, _>(account_id.to_string())
            .bind::<Text, _>(signature)
            .load::<TokenModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch token: {}", e))
            })?;

        if tokens.is_empty() {
            return Ok(FetchResponseKind::NotFound(None));
        }

        let now = Local::now();

        let valid_tokens: Vec<Token> = tokens
            .into_iter()
            .filter_map(|token| {
                let meta: UserAccountConnectionString =
                    match serde_json::from_str(&token.meta) {
                        Ok(m) => m,
                        Err(err) => {
                            error!("Error parsing token meta: {}", err);
                            return None;
                        }
                    };

                let expiration = naive_timestamp_from_text(&token.expiration)
                    .ok()?
                    .and_local_timezone(Local)
                    .unwrap();

                if expiration < now {
                    return None;
                }

                Some(Token::new(
                    Some(token.id),
                    expiration,
                    MultiTypeMeta::UserAccountConnectionString(meta),
                ))
            })
            .collect();

        match valid_tokens.len() {
            0 => Ok(FetchResponseKind::NotFound(Some(
                "Token not found".to_string(),
            ))),
            1 => Ok(FetchResponseKind::Found(valid_tokens[0].clone())),
            _ => fetching_err("Multiple tokens found")
                .with_code(NativeErrorCodes::MYC00020)
                .as_error(),
        }
    }

    #[tracing::instrument(
        name = "list_connection_strings_by_account_id",
        skip_all
    )]
    async fn list_connection_strings_by_account_id(
        &self,
        account_id: Uuid,
    ) -> Result<FetchManyResponseKind<PublicConnectionStringInfo>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let sql = r#"
            SELECT id, innerId, accountId, email, name, expiration, createdAt, scope
            FROM public_connection_string_info
            WHERE accountId = ?
            ORDER BY id DESC
        "#;

        let rows = diesel::sql_query(sql)
            .bind::<Text, _>(account_id.to_string())
            .load::<PublicConnectionStringInfoModel>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to fetch connection strings: {}",
                    e
                ))
            })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let connection_strings: Result<
            Vec<PublicConnectionStringInfo>,
            MappedErrors,
        > = rows
            .into_iter()
            .map(map_public_connection_string_info_model_to_dto)
            .collect();

        match connection_strings {
            Ok(items) => Ok(FetchManyResponseKind::Found(items)),
            Err(e) => Err(e),
        }
    }
}
