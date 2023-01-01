use crate::domain::{
    dtos::{
        email::Email, guest::GuestUser, message::Message, profile::Profile,
    },
    entities::{AccountFetching, GuestUserRegistration, MessageSending},
};

use chrono::Local;
use clean_base::{
    dtos::enums::ParentEnum,
    entities::default_response::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use log::{info, warn};
use uuid::Uuid;

/// Guest a user to perform actions into an account.
pub async fn guest_user(
    profile: Profile,
    email: Email,
    role: Uuid,
    target_account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
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
    // ? Check if account has subscription type
    //
    // Check if the target account is a subscription account.
    // ? -----------------------------------------------------------------------

    match account_fetching_repo.get(target_account_id).await {
        Err(err) => return Err(err),
        Ok(res) => match res {
            FetchResponseKind::NotFound(id) => {
                return Err(use_case_err(
                    format!("Target account not found: {:?}", id.unwrap()),
                    None,
                    None,
                ))
            }
            FetchResponseKind::Found(res) => match res.account_type {
                ParentEnum::Id(id) => {
                    return Err(use_case_err(
                        format!(
                            "Could not check the account type validity: {}",
                            id
                        ),
                        None,
                        None,
                    ))
                }
                ParentEnum::Record(res) => {
                    if !res.is_subscription {
                        return Err(use_case_err(
                            format!(
                                "Invalid account ({:?}). Only subscription 
                                accounts should receive guesting.",
                                res.id
                            ),
                            None,
                            None,
                        ));
                    }
                }
            },
        },
    }

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    let guest_user = guest_user_registration_repo
        .get_or_create(
            GuestUser {
                id: None,
                email: email.to_owned(),
                guest_role: ParentEnum::Id(role),
                created: Local::now(),
                updated: None,
                accounts: None,
            },
            target_account_id,
        )
        .await;

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    if guest_user.is_ok() {
        match message_sending_repo
            .send(Message {
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
