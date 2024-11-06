use crate::{
    prisma::token as token_model, repositories::connector::get_client,
};

use async_trait::async_trait;
use chrono::{DateTime, Local};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        route_type::PermissionedRoles,
        token::{
            AccountScopedConnectionStringMeta,
            AccountWithPermissionedRolesScope, ConnectionStringBean,
            MultiTypeMeta, Token,
        },
    },
    entities::TokenFetching,
};
use mycelium_base::{
    entities::FetchResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use prisma_client_rust::{PrismaValue, Raw};
use serde_json::from_value;
use shaku::Component;
use std::process::id as process_id;
use tracing::error;
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = TokenFetching)]
pub struct TokenFetchingSqlDbRepository {}

#[async_trait]
impl TokenFetching for TokenFetchingSqlDbRepository {
    async fn get_connection_string_by_account_with_permissioned_roles_scope(
        &self,
        scope: AccountWithPermissionedRolesScope,
    ) -> Result<FetchResponseKind<Token, String>, MappedErrors> {
        // ? -------------------------------------------------------------------
        // ? Try to build the prisma client
        // ? -------------------------------------------------------------------

        let tmp_client = get_client().await;

        let client = match tmp_client.get(&process_id()) {
            None => {
                return fetching_err(String::from(
                    "Prisma Client error. Could not fetch client.",
                ))
                .with_code(NativeErrorCodes::MYC00001)
                .as_error()
            }
            Some(res) => res,
        };

        // ? -------------------------------------------------------------------
        // ? Try to fetch token
        // ? -------------------------------------------------------------------

        let beans = scope.get_scope_beans();

        let tenant_id: Uuid = match beans.iter().find_map(|bean| {
            if let &ConnectionStringBean::TID(tenant_id) = bean {
                return Some(tenant_id);
            }

            None
        }) {
            Some(tenant_id) => tenant_id,
            _ => return Ok(FetchResponseKind::NotFound(None)),
        };

        let account_id: Uuid = match beans.iter().find_map(|bean| {
            if let &ConnectionStringBean::AID(account_id) = bean {
                return Some(account_id);
            }

            None
        }) {
            Some(account_id) => account_id,
            _ => return Ok(FetchResponseKind::NotFound(None)),
        };

        let signature: String = match beans.iter().find_map(|bean| {
            if let ConnectionStringBean::SIG(signature) = bean {
                return Some(signature);
            }

            None
        }) {
            Some(signature) => signature.to_owned(),
            _ => return Ok(FetchResponseKind::NotFound(None)),
        };

        let permissioned_roles: PermissionedRoles =
            match beans.iter().find_map(|bean| match bean {
                ConnectionStringBean::PR(permissioned_roles) => {
                    return Some(permissioned_roles);
                }
                _ => None,
            }) {
                Some(permissioned_roles) => permissioned_roles.to_owned(),
                _ => return Ok(FetchResponseKind::NotFound(None)),
            };

        let token_data: Vec<token_model::Data> = match client
            ._query_raw(Raw::new(
                "SELECT id, expiration, meta FROM token WHERE EXISTS (SELECT 1 FROM jsonb_array_elements(meta->'scope') AS elem WHERE elem->>'tid' = {} AND elem->>'aid' = {} AND elem->>'sig' = {})",
                vec![
                    PrismaValue::String(tenant_id.to_string().to_owned()),
                    PrismaValue::String(account_id.to_string().to_owned()),
                    PrismaValue::String(signature.to_owned()),
                ],
            ))
            .exec()
            .await {
                Ok(token_option) => token_option,
                Err(err) => {
                    error!("Error fetching token: {err}", );

                    return Ok(FetchResponseKind::NotFound(None))
                }
            };

        if token_data.len() == 0 {
            return Ok(FetchResponseKind::NotFound(None));
        }

        let tokens: Vec<Token> = token_data
            .iter()
            .filter_map(|data| {
                let meta: AccountScopedConnectionStringMeta =
                    match from_value(data.meta.to_owned()) {
                        Ok(meta) => meta,
                        Err(err) => {
                            error!("Error parsing token meta: {err}",);

                            return None;
                        }
                    };

                let expiration: DateTime<Local> = data.expiration.into();

                if expiration < chrono::Utc::now() {
                    return None;
                }

                let meta_permissioned_roles = match meta.get_permissioned_roles() {
                    Some(permissioned_roles) => permissioned_roles,
                    None => {
                        error!("Error parsing token meta: permissioned roles not found");

                        return None;
                    }
                };

                if meta_permissioned_roles.iter().all(|(role, permission)| {
                    !permissioned_roles.contains(&(role.to_owned(), permission.to_owned()))
                }) {
                    return None;
                }

                Some(Token::new(
                    data.id.try_into().unwrap(),
                    data.expiration.into(),
                    MultiTypeMeta::AccountScopedConnectionString(meta),
                ))
            })
            .collect();

        if tokens.len() == 0 {
            return Ok(FetchResponseKind::NotFound(None));
        }

        if tokens.len() > 1 {
            return fetching_err(String::from("Multiple tokens found"))
                .with_code(NativeErrorCodes::MYC00020)
                .as_error();
        }

        Ok(FetchResponseKind::Found(tokens[0].to_owned()))
    }
}
