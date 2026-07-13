use crate::{
    models::instance_settings::InstanceSettingsRow,
    repositories::parse_optional_written_by,
    types::{json_from_text, naive_timestamp_from_text},
};

use chrono::Local;
use myc_core::domain::dtos::instance_settings::InstanceSetting;
use mycelium_base::utils::errors::MappedErrors;

pub(crate) fn map_instance_settings_row_to_dto(
    row: InstanceSettingsRow,
) -> Result<InstanceSetting, MappedErrors> {
    let created = naive_timestamp_from_text(&row.created)?
        .and_local_timezone(Local)
        .unwrap();

    let updated = row
        .updated
        .as_deref()
        .map(naive_timestamp_from_text)
        .transpose()?
        .map(|dt| dt.and_local_timezone(Local).unwrap());

    let value = json_from_text(&row.value)?;

    let created_by =
        row.created_by.as_deref().map(json_from_text).transpose()?;

    let updated_by =
        row.updated_by.as_deref().map(json_from_text).transpose()?;

    Ok(InstanceSetting {
        key: row.key,
        value,
        created_by: parse_optional_written_by(created_by),
        updated_by: parse_optional_written_by(updated_by),
        created,
        updated,
    })
}
