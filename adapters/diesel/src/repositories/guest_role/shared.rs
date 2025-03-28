use crate::models::guest_role::GuestRole as GuestRoleModel;

use chrono::Local;
use myc_core::domain::dtos::guest_role::{GuestRole, Permission};

pub(super) fn map_model_to_dto(model: GuestRoleModel) -> GuestRole {
    GuestRole {
        id: Some(model.id),
        name: model.name,
        slug: model.slug,
        description: model.description,
        permission: Permission::from_i32(model.permission),
        children: None,
        system: model.system,
        created: model.created.and_local_timezone(Local).unwrap(),
        updated: model
            .updated
            .map(|dt| dt.and_local_timezone(Local).unwrap()),
    }
}
