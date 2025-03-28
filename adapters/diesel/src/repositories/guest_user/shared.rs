use crate::models::{
    guest_role::GuestRole as GuestRoleModel,
    guest_user::GuestUser as GuestUserModel,
};

use chrono::Local;
use myc_core::domain::dtos::{
    email::Email,
    guest_role::{GuestRole, Permission},
    guest_user::GuestUser,
};
use mycelium_base::dtos::Parent;

pub(super) fn map_model_to_dto(
    model: GuestUserModel,
    guest_role: Option<GuestRoleModel>,
) -> GuestUser {
    GuestUser {
        id: Some(model.id),
        email: Email::from_string(model.email).unwrap(),
        guest_role: match guest_role {
            Some(role) => Parent::Record(GuestRole {
                id: Some(role.id),
                name: role.name,
                slug: role.slug,
                description: role.description,
                permission: Permission::from_i32(role.permission),
                children: None,
                system: role.system,
                created: role.created.and_local_timezone(Local).unwrap(),
                updated: role
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap()),
            }),
            None => Parent::Id(model.guest_role_id),
        },
        accounts: None,
        created: model.created,
        updated: model.updated,
        was_verified: model.was_verified,
    }
}
