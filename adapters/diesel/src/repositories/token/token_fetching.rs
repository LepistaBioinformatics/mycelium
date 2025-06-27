use crate::models::{config::DbPoolProvider, token::Token as TokenModel};

use async_trait::async_trait;
use chrono::Local;
use diesel::RunQueryDsl;
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        token::{
            ConnectionStringBean, MultiTypeMeta, Token,
            UserAccountConnectionString, UserAccountScope,
        },
    },
    entities::TokenFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::sync::Arc;
use tracing::error;

#[derive(Component)]
#[shaku(interface = TokenFetching)]
pub struct TokenFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
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

        let sql = format!(
            r#"
            SELECT id, expiration, meta
            FROM token
            WHERE EXISTS (
                SELECT 1
                FROM jsonb_array_elements(meta->'scope') AS elem
                WHERE elem->>'aid' = '{}'
            )
            AND EXISTS (
                SELECT 1
                FROM jsonb_array_elements(meta->'scope') AS elem
                WHERE elem->>'sig' = '{}'
            )"#,
            account_id, signature
        );

        let tokens =
            diesel::sql_query(sql)
                .load::<TokenModel>(conn)
                .map_err(|e| {
                    fetching_err(format!("Failed to fetch token: {}", e))
                })?;

        if tokens.is_empty() {
            return Ok(FetchResponseKind::NotFound(None));
        }

        let valid_tokens: Vec<Token> = tokens
            .into_iter()
            .filter_map(|token| {
                let meta: UserAccountConnectionString =
                    match from_value(token.meta) {
                        Ok(m) => m,
                        Err(err) => {
                            error!("Error parsing token meta: {}", err);
                            return None;
                        }
                    };

                let expiration =
                    token.expiration.and_local_timezone(Local).unwrap();
                if expiration < chrono::Utc::now().with_timezone(&Local) {
                    return None;
                }

                Some(Token::new(
                    token.id.try_into().unwrap(),
                    token.expiration.and_local_timezone(Local).unwrap(),
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
}
