use crate::models::public_connection_string_info::PublicConnectionStringInfoModel;

use chrono::Local;
use myc_core::domain::dtos::{
    email::Email,
    token::{ConnectionStringBean, PublicConnectionStringInfo},
};
use mycelium_base::utils::errors::{fetching_err, MappedErrors};
use serde_json::{from_value, Value as JsonValue};
use uuid::Uuid;

pub(crate) fn map_public_connection_string_info_model_to_dto(
    model: PublicConnectionStringInfoModel,
) -> Result<PublicConnectionStringInfo, MappedErrors> {
    // Parse inner_id from JSONB
    let inner_id = match model.inner_id {
        Some(JsonValue::String(s)) => Uuid::parse_str(&s).map_err(|e| {
            fetching_err(format!("Failed to parse inner_id: {}", e))
        })?,
        Some(v) => {
            return fetching_err(format!(
                "Invalid inner_id format: expected string, got {:?}",
                v
            ))
            .as_error();
        }
        None => {
            return fetching_err("inner_id is required but was null")
                .as_error();
        }
    };

    // Parse account_id from JSONB
    let account_id = match model.account_id {
        Some(JsonValue::String(s)) => Uuid::parse_str(&s).map_err(|e| {
            fetching_err(format!("Failed to parse account_id: {}", e))
        })?,
        Some(v) => {
            return fetching_err(format!(
                "Invalid account_id format: expected string, got {:?}",
                v
            ))
            .as_error();
        }
        None => {
            return fetching_err("account_id is required but was null")
                .as_error();
        }
    };

    // Parse email from JSONB
    let email_str = match model.email {
        Some(JsonValue::String(s)) => s,
        Some(v) => {
            return fetching_err(format!(
                "Invalid email format: expected string, got {:?}",
                v
            ))
            .as_error();
        }
        None => {
            return fetching_err("email is required but was null").as_error();
        }
    };
    let email = Email::from_string(email_str)
        .map_err(|e| fetching_err(format!("Failed to parse email: {}", e)))?;

    // Parse name from JSONB
    let name = match model.name {
        Some(JsonValue::String(s)) => s,
        Some(v) => {
            return fetching_err(format!(
                "Invalid name format: expected string, got {:?}",
                v
            ))
            .as_error();
        }
        None => {
            return fetching_err("name is required but was null").as_error();
        }
    };

    // Parse created_at from JSONB
    let created_at = match model.created_at {
        Some(JsonValue::String(s)) => chrono::DateTime::parse_from_rfc3339(&s)
            .map_err(|e| {
                fetching_err(format!("Failed to parse created_at: {}", e))
            })?
            .with_timezone(&Local),
        Some(v) => {
            return fetching_err(format!(
                "Invalid created_at format: expected string, got {:?}",
                v
            ))
            .as_error();
        }
        None => {
            return fetching_err("created_at is required but was null")
                .as_error();
        }
    };

    // Parse scope from JSONB array
    let scope = match model.scope {
        Some(JsonValue::Array(arr)) => {
            arr.into_iter()
                .map(|v| {
                    // Each element in the scope array should be a ConnectionStringBean
                    from_value::<ConnectionStringBean>(v).map_err(|e| {
                        fetching_err(format!(
                            "Failed to parse scope bean: {}",
                            e
                        ))
                    })
                })
                .collect::<Result<Vec<ConnectionStringBean>, MappedErrors>>()?
        }
        Some(v) => {
            return fetching_err(format!(
                "Invalid scope format: expected array, got {:?}",
                v
            ))
            .as_error();
        }
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
        expiration: model.expiration.and_local_timezone(Local).unwrap(),
        created_at,
        scope,
    })
}
