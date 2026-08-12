use crate::{
    models::guest_role::GuestRole as GuestRoleModel,
    repositories::account::created_at_from_text, types::uuid_from_text,
};

use myc_core::domain::dtos::guest_role::{GuestRole, Permission};

pub(crate) fn map_model_to_dto(model: GuestRoleModel) -> GuestRole {
    GuestRole {
        id: Some(uuid_from_text(&model.id).unwrap()),
        name: model.name,
        slug: model.slug,
        description: model.description,
        permission: Permission::from_i32(model.permission),
        children: None,
        system: model.system,
        created: created_at_from_text(&model.created),
        updated: model.updated.map(|dt| created_at_from_text(&dt)),
    }
}
