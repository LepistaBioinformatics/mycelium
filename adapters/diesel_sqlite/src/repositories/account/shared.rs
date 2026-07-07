use crate::{
    models::account::Account as AccountModel,
    repositories::parse_optional_written_by,
    types::{json_from_text, naive_timestamp_from_text, uuid_from_text},
};

use chrono::{DateTime, Local};
use myc_core::domain::dtos::account::{Account, AccountMetaKey, VerboseStatus};
use mycelium_base::dtos::Children;
use std::{collections::HashMap, str::FromStr};

/// Reconstructs the `DateTime<Local>` the domain expects from a naive
/// `created`/`updated` TEXT column, mirroring the postgres repositories'
/// `.and_local_timezone(Local)` round-trip (see `sqlite::types` for the
/// behavioral note on why this is a reinterpretation, not a UTC->Local
/// conversion).
pub(crate) fn created_at_from_text(value: &str) -> DateTime<Local> {
    naive_timestamp_from_text(value)
        .unwrap()
        .and_local_timezone(Local)
        .unwrap()
}

pub(crate) fn map_account_model_to_dto(model: AccountModel) -> Account {
    Account {
        id: Some(uuid_from_text(&model.id).unwrap()),
        name: model.name,
        slug: model.slug,
        tags: None,
        is_active: model.is_active,
        is_checked: model.is_checked,
        is_archived: model.is_archived,
        is_deleted: model.is_deleted,
        verbose_status: Some(VerboseStatus::from_flags(
            model.is_active,
            model.is_checked,
            model.is_archived,
            model.is_deleted,
        )),
        is_system_account: model.is_default,
        owners: Children::Records(vec![]),
        account_type: serde_json::from_str(&model.account_type).unwrap(),
        guest_users: None,
        created_at: created_at_from_text(&model.created),
        created_by: parse_optional_written_by(
            model.created_by.map(|s| json_from_text(&s).unwrap()),
        ),
        updated_at: model.updated.map(|dt| created_at_from_text(&dt)),
        updated_by: parse_optional_written_by(
            model.updated_by.map(|s| json_from_text(&s).unwrap()),
        ),
        meta: model.meta.map(|m| {
            serde_json::from_value::<HashMap<String, String>>(
                json_from_text(&m).unwrap(),
            )
            .unwrap()
            .iter()
            .map(|(k, v)| (AccountMetaKey::from_str(k).unwrap(), v.to_string()))
            .collect()
        }),
    }
}
