use crate::{
    domain::{
        dtos::{
            account::Account, email::Email, guest::GuestUser, message::Message,
            user::User,
        },
        entities::{
            AccountRegistration, GuestUserRegistration, MessageSending,
        },
    },
    models::AccountLifeCycle,
    use_cases::roles::shared::account::get_or_create_role_related_account,
};

use chrono::Local;
use mycelium_base::{
    dtos::{Children, Parent},
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use tracing::{info, warn};
use uuid::Uuid;

/// Guest a user to a default account
///
/// This method should be called from webhooks to propagate a new user to a
/// default account.
#[tracing::instrument(name = "guest_to_default_account", skip_all)]
pub async fn guest_to_default_account(
    role_id: Uuid,
    account: Account,
    tenant_id: Uuid,
    token_secret: AccountLifeCycle,
    account_registration_repo: Box<&dyn AccountRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch default account
    // ? -----------------------------------------------------------------------

    let default_subscription_account = match get_or_create_role_related_account(
        tenant_id,
        role_id,
        account_registration_repo,
    )
    .await?
    {
        GetOrCreateResponseKind::NotCreated(account, _) => account,
        GetOrCreateResponseKind::Created(account) => account,
    };

    // ? -----------------------------------------------------------------------
    // ? Persist changes
    // ? -----------------------------------------------------------------------

    let guest_email = match account.owners {
        Children::Ids(_) => {
            return use_case_err("Invalid account owner".to_string()).as_error()
        }
        Children::Records(owners) => owners
            .into_iter()
            .filter(|owner| owner.is_principal())
            .collect::<Vec<User>>()
            .first()
            .unwrap()
            .email
            .to_owned(),
    };

    match guest_user_registration_repo
        .get_or_create(
            GuestUser {
                id: None,
                email: guest_email.to_owned(),
                guest_role: Parent::Id(role_id),
                created: Local::now(),
                updated: None,
                accounts: None,
            },
            match default_subscription_account.id {
                None => {
                    warn!(
                        "Default account maybe invalid. ID not found: {:?}",
                        default_subscription_account
                    );

                    return use_case_err("Invalid default account".to_string())
                        .as_error();
                }
                Some(id) => id,
            },
        )
        .await?
    {
        GetOrCreateResponseKind::Created(guest_user) => {
            info!("Guest user created: {}", guest_user.email.get_email());
        }
        GetOrCreateResponseKind::NotCreated(_, msg) => {
            return use_case_err(format!("Guest user not created: {msg}"))
                .as_error()
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    match message_sending_repo
        .send(Message {
            from: Email::from_string(
                token_secret.noreply_email.get_or_error()?,
            )?,
            to: guest_email,
            cc: None,
            subject: String::from("Congratulations! New collaboration invite"),
            message_head: Some(
                "Your account was successfully created".to_string(),
            ),
            message_body: "Your account was activated with success."
                .to_string(),
            message_footer: None,
        })
        .await
    {
        Err(err) => {
            warn!("Guesting to default account not occurred: {:?}", err)
        }
        Ok(res) => {
            info!("Guesting to default account successfully done: {:?}", res)
        }
    };

    Ok(())
}
