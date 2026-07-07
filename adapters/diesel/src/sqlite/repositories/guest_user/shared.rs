use crate::sqlite::{
    models::{
        guest_role::GuestRole as GuestRoleModel,
        guest_user::GuestUser as GuestUserModel,
    },
    repositories::account::created_at_from_text,
    types::{timestamp_from_text, uuid_from_text},
};

use myc_core::domain::dtos::{
    email::Email,
    guest_role::{GuestRole, Permission},
    guest_user::GuestUser,
};
use mycelium_base::dtos::Parent;

/// Unlike most sqlite models, `guest_user.created`/`updated` are a genuine
/// timezone-aware round trip (mirrors the postgres model, which stores
/// `DateTime<Local>` directly rather than the naive-reinterpretation
/// convention used elsewhere) -- see `sqlite::types::timestamp_from_text`.
pub(crate) fn map_model_to_dto(
    model: GuestUserModel,
    guest_role: Option<GuestRoleModel>,
) -> GuestUser {
    GuestUser {
        id: Some(uuid_from_text(&model.id).unwrap()),
        email: Email::from_string(model.email).unwrap(),
        guest_role: match guest_role {
            Some(role) => Parent::Record(GuestRole {
                id: Some(uuid_from_text(&role.id).unwrap()),
                name: role.name,
                slug: role.slug,
                description: role.description,
                permission: Permission::from_i32(role.permission),
                children: None,
                system: role.system,
                created: created_at_from_text(&role.created),
                updated: role.updated.map(|dt| created_at_from_text(&dt)),
            }),
            None => Parent::Id(uuid_from_text(&model.guest_role_id).unwrap()),
        },
        accounts: None,
        created: timestamp_from_text(&model.created)
            .unwrap()
            .with_timezone(&chrono::Local),
        updated: model.updated.map(|dt| {
            timestamp_from_text(&dt)
                .unwrap()
                .with_timezone(&chrono::Local)
        }),
        was_verified: model.was_verified,
    }
}
