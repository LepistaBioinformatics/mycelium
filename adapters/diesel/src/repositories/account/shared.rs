use crate::models::account::Account as AccountModel;

use chrono::Local;
use myc_core::domain::dtos::account::{Account, VerboseStatus};
use mycelium_base::dtos::Children;
use serde_json::from_value;
use std::str::FromStr;
use uuid::Uuid;

pub(super) fn map_account_model_to_dto(model: AccountModel) -> Account {
    Account {
        id: Some(Uuid::from_str(&model.id).unwrap()),
        name: model.name,
        slug: model.slug,
        tags: None,
        is_active: model.is_active,
        is_checked: model.is_checked,
        is_archived: model.is_archived,
        verbose_status: Some(VerboseStatus::from_flags(
            model.is_active,
            model.is_checked,
            model.is_archived,
        )),
        is_default: model.is_default,
        owners: Children::Records(vec![]),
        account_type: from_value(model.account_type).unwrap(),
        guest_users: None,
        created: model.created.and_local_timezone(Local).unwrap(),
        updated: model
            .updated
            .map(|dt| dt.and_local_timezone(Local).unwrap()),
        meta: None,
    }
}
