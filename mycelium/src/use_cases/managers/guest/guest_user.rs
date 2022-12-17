use crate::domain::{
    dtos::{
        email::EmailDTO, guest::GuestUserDTO, message::MessageDTO,
        profile::ProfileDTO,
    },
    entities::{
        manager::guest_user_registration::GuestUserRegistration,
        shared::send_message::MessageSending,
    },
};

use chrono::Local;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use log::{info, warn};
use uuid::Uuid;

/// Guest a user to perform actions into an account.
pub async fn guest_user(
    profile: ProfileDTO,
    email: EmailDTO,
    role: Uuid,
    target_account_id: Uuid,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<GetOrCreateResponseKind<GuestUserDTO>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    if !profile.is_manager {
        return Err(use_case_err(
            "The current user has no sufficient privileges to guest accounts."
                .to_string(),
            Some(true),
            None,
        ));
    };

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    let guest_user = guest_user_registration_repo
        .get_or_create(
            GuestUserDTO {
                id: None,
                email: email.to_owned(),
                role: ParentEnum::Id(role),
                created: Local::now(),
                updated: None,
            },
            target_account_id,
        )
        .await;

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    if guest_user.is_ok() {
        match message_sending_repo
            .send(MessageDTO {
                from: email.to_owned(),
                to: email,
                cc: None,
                subject: String::from("New collaboration invite."),
                message_head: None,
                message_body: format!(
                    "You were invited to collaborate with account: {}",
                    target_account_id.to_string()
                ),
                message_footer: None,
            })
            .await
        {
            Err(err) => warn!("Confirmation message not send: {:?}", err),
            Ok(res) => info!("Confirmation message send done: {:?}", res),
        };
    }

    // ? -----------------------------------------------------------------------
    // ? Send the guesting response
    // ? -----------------------------------------------------------------------

    guest_user
}