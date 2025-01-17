use crate::models::guest_role::GuestRole as GuestRoleModel;
use myc_core::domain::dtos::guest_role::{GuestRole, Permission};
use std::str::FromStr;
use uuid::Uuid;

pub(super) fn map_model_to_dto(model: GuestRoleModel) -> GuestRole {
    GuestRole {
        id: Some(Uuid::from_str(&model.id).unwrap()),
        name: model.name,
        slug: model.slug,
        description: model.description,
        permission: Permission::from_i32(model.permission),
        children: None,
    }
}
