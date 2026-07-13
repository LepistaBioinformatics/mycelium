use crate::{
    models::instance_settings::InstanceSettingsRow,
    repositories::parse_optional_written_by,
};

use chrono::Local;
use myc_core::domain::dtos::instance_settings::InstanceSetting;

pub(crate) fn map_instance_settings_row_to_dto(
    row: InstanceSettingsRow,
) -> InstanceSetting {
    InstanceSetting {
        key: row.key,
        value: row.value,
        created_by: parse_optional_written_by(row.created_by),
        updated_by: parse_optional_written_by(row.updated_by),
        created: row.created.and_local_timezone(Local).unwrap(),
        updated: row.updated.map(|dt| dt.and_local_timezone(Local).unwrap()),
    }
}
