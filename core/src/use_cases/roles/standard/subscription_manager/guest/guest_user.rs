use crate::domain::{
    actors::ActorName,
    dtos::{
        account_type::AccountTypeV2, email::Email, guest::GuestUser,
        message::Message, native_error_codes::NativeErrorCodes,
        profile::Profile,
    },
    entities::{AccountFetching, GuestUserRegistration, MessageSending},
};

use chrono::Local;
use mycelium_base::{
    dtos::Parent,
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::{info, warn};
use uuid::Uuid;

/// Guest a user to perform actions into an account.
#[tracing::instrument(
    name = "guest_user",
    fields(profile_id = %profile.acc_id),
    skip_all
)]
pub async fn guest_user(
    profile: Profile,
    tenant_id: Uuid,
    email: Email,
    role_id: Uuid,
    target_account_id: Uuid,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_create_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Check if account has subscription type
    //
    // Check if the target account is a subscription account.
    // ? -----------------------------------------------------------------------

    match account_fetching_repo.get(target_account_id, related_accounts).await? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Target account not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
        FetchResponseKind::Found(account) => match account.account_type {
            AccountTypeV2::Subscription { .. } => (),
            _ => {
                return use_case_err(
                    "Invalid account. Only subscription accounts should receive guesting."
                )
                .as_error()
            }
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
                guest_role: Parent::Id(role_id),
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
