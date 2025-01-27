use crate::models::guest_user::GuestUser as GuestUserModel;
use myc_core::domain::dtos::{email::Email, guest_user::GuestUser};
use mycelium_base::dtos::Parent;
use std::str::FromStr;
use uuid::Uuid;

pub(super) fn map_model_to_dto(model: GuestUserModel) -> GuestUser {
    GuestUser {
        id: Some(Uuid::from_str(&model.id).unwrap()),
        email: Email::from_string(model.email).unwrap(),
        guest_role: Parent::Id(Uuid::from_str(&model.guest_role_id).unwrap()),
        accounts: None,
        created: model.created,
        updated: model.updated,
        was_verified: model.was_verified,
    }
}
