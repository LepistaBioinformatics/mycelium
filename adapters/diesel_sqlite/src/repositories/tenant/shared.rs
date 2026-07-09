use crate::{
    models::tenant::Tenant as TenantModel,
    types::{json_array_from_text, json_from_text, uuid_from_text},
};

use myc_core::domain::dtos::tenant::{Tenant, TenantMetaKey, TenantStatus};
use mycelium_base::dtos::Children;
use std::{collections::HashMap, str::FromStr};

/// Decodes the `status` column (a JSON-array-of-statuses TEXT value) into the
/// domain's `Vec<TenantStatus>`, treating a missing column the same as an
/// empty list -- matching every postgres read site
/// (`.unwrap_or_default().into_iter().map(from_value)`).
pub(crate) fn decode_status(status: Option<String>) -> Vec<TenantStatus> {
    let Some(status) = status else {
        return vec![];
    };

    json_array_from_text(&status)
        .unwrap()
        .into_iter()
        .map(|s| serde_json::from_value(s).unwrap())
        .collect()
}

pub(crate) fn map_tenant_model_to_dto(model: TenantModel) -> Tenant {
    Tenant {
        id: Some(uuid_from_text(&model.id).unwrap()),
        name: model.name,
        description: model.description,
        owners: Children::Records(vec![]),
        manager: None,
        tags: None,
        meta: model.meta.map(|m| {
            serde_json::from_value::<HashMap<String, String>>(
                json_from_text(&m).unwrap(),
            )
            .unwrap()
            .iter()
            .map(|(k, v)| (TenantMetaKey::from_str(k).unwrap(), v.to_string()))
            .collect()
        }),
        status: Some(decode_status(model.status)),
        created: crate::repositories::account::created_at_from_text(
            &model.created,
        ),
        updated: model
            .updated
            .map(|dt| crate::repositories::account::created_at_from_text(&dt)),
    }
}
