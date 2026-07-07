use crate::sqlite::models::public_connection_string_info::PublicConnectionStringInfoModel;

use myc_core::domain::dtos::token::{
    ConnectionStringBean, PublicConnectionStringInfo,
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use uuid::Uuid;

/// Rebuilds `PublicConnectionStringInfo` from the SQLite view row. Unlike
/// postgres (every column is native `Jsonb`), `json_extract` already unwraps
/// scalar values (`innerId`, `accountId`, `name`, `createdAt`) to plain text;
/// only the compound values (`email`, `scope`) still need JSON parsing.
pub(crate) fn map_public_connection_string_info_model_to_dto(
    model: PublicConnectionStringInfoModel,
) -> Result<PublicConnectionStringInfo, MappedErrors> {
    let inner_id = match model.inner_id {
        Some(s) => Uuid::parse_str(&s).map_err(|e| {
            fetching_err(format!("Failed to parse inner_id: {}", e))
        })?,
        None => {
            return fetching_err("inner_id is required but was null")
                .as_error();
        }
    };

    let account_id = match model.account_id {
        Some(s) => Uuid::parse_str(&s).map_err(|e| {
            fetching_err(format!("Failed to parse account_id: {}", e))
        })?,
        None => {
            return fetching_err("account_id is required but was null")
                .as_error();
        }
    };

    let email = match model.email {
        Some(s) => serde_json::from_str(&s).map_err(|e| {
            fetching_err(format!("Failed to parse email: {}", e))
        })?,
        None => {
            return fetching_err("email is required but was null").as_error();
        }
    };

    let name = match model.name {
        Some(s) => s,
        None => {
            return fetching_err("name is required but was null").as_error();
        }
    };

    let created_at = match model.created_at {
        Some(s) => crate::sqlite::types::timestamp_from_text(&s)
            .map_err(|e| {
                fetching_err(format!("Failed to parse created_at: {e}"))
            })?
            .with_timezone(&chrono::Local),
        None => {
            return fetching_err("created_at is required but was null")
                .as_error();
        }
    };

    let scope: Vec<ConnectionStringBean> = match model.scope {
        Some(s) => serde_json::from_str(&s).map_err(|e| {
            fetching_err(format!("Failed to parse scope: {}", e))
        })?,
        None => {
            return fetching_err("scope is required but was null").as_error();
        }
    };

    Ok(PublicConnectionStringInfo {
        id: model.id as u32,
        inner_id,
        account_id,
        email,
        name,
        expiration: crate::sqlite::types::naive_timestamp_from_text(
            &model.expiration,
        )
        .unwrap()
        .and_local_timezone(chrono::Local)
        .unwrap(),
        created_at,
        scope,
    })
}
