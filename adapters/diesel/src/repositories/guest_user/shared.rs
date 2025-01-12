use crate::models::guest_user::GuestUser as GuestUserModel;
use myc_core::domain::dtos::{email::Email, guest_user::GuestUser};
use mycelium_base::dtos::Parent;

pub(super) fn map_model_to_dto(model: GuestUserModel) -> GuestUser {
    GuestUser {
        id: Some(model.id),
        email: Email::from_string(model.email).unwrap(),
        guest_role: Parent::Id(model.guest_role_id),
        accounts: None,
        created: model.created,
        updated: model.updated,
        was_verified: model.was_verified,
    }
}
