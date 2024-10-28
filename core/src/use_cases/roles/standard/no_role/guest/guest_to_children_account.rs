//use crate::domain::{
//    dtos::{email::Email, profile::Profile},
//    entities::{AccountFetching, GuestUserRegistration, MessageSending},
//};
//
//use uuid::Uuid;

/// Guest users to collaborate to an account children of a role which I have
/// guest to collaborate.
///
/// This action should be allowed only to accounts that contains registered
/// children accounts already registered.
#[tracing::instrument(name = "guest_to_children_account", skip_all)]
pub async fn guest_to_children_account(//profile: Profile,
    //email: Email,
    //role: Uuid,
    //target_account_id: Uuid,
    //account_fetching_repo: Box<&dyn AccountFetching>,
    //guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    //message_sending_repo: Box<&dyn MessageSending>,
) {
    unimplemented!("guest_to_children_account")
}
