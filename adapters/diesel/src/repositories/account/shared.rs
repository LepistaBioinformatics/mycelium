use crate::models::account::Account as AccountModel;

use chrono::Local;
use myc_core::domain::dtos::account::{
    Account, AccountMetaKey, Modifier, VerboseStatus,
};
use mycelium_base::dtos::Children;
use serde_json::{from_value, json};
use std::{collections::HashMap, str::FromStr};

pub(crate) fn map_account_model_to_dto(model: AccountModel) -> Account {
    Account {
        id: Some(model.id),
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
        is_default: model.is_default,
        owners: Children::Records(vec![]),
        account_type: from_value(model.account_type).unwrap(),
        guest_users: None,
        created_at: model.created.and_local_timezone(Local).unwrap(),
        created_by: model.created_by.map(|m| from_value(m).unwrap()),
        updated_at: model
            .updated
            .map(|dt| dt.and_local_timezone(Local).unwrap()),
        updated_by: model
            .updated_by
            .map(|m| {
                //
                // Check if the Value is a empty object
                //
                if m == json!({}) {
                    None
                } else {
                    let modifier: Modifier = from_value(m).unwrap();
                    Some(modifier)
                }
            })
            .flatten(),
        meta: model.meta.map(|m| {
            serde_json::from_value::<HashMap<String, String>>(m)
                .unwrap()
                .iter()
                .map(|(k, v)| {
                    (AccountMetaKey::from_str(k).unwrap(), v.to_string())
                })
                .collect()
        }),
    }
}
