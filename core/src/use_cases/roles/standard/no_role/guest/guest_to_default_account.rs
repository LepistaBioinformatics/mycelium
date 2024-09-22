use crate::{
    domain::{
        dtos::{
            account::{Account, AccountTypeEnum},
            email::Email,
            guest::GuestUser,
            message::Message,
            session_token::TokenSecret,
            user::User,
        },
        entities::{
            AccountRegistration, AccountTypeRegistration,
            GuestUserRegistration, MessageSending,
        },
    },
    use_cases::roles::shared::{
        account::get_or_create_default_subscription_account,
        account_type::get_or_create_default_account_types,
    },
};

use chrono::Local;
use log::{info, warn};
use mycelium_base::{
    dtos::{Children, Parent},
    entities::GetOrCreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};
use uuid::Uuid;

pub async fn guest_to_default_account(
    role: Uuid,
    account: Account,
    token_secret: TokenSecret,
    account_type_registration_repo: Box<&dyn AccountTypeRegistration>,
    account_registration_repo: Box<&dyn AccountRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Fetch account type
    //
    // Get or create the default account-type.
    // ? -----------------------------------------------------------------------

    let account_type = match get_or_create_default_account_types(
        AccountTypeEnum::Subscription,
        None,
        None,
        account_type_registration_repo,
    )
    .await?
    {
        GetOrCreateResponseKind::NotCreated(account_type, _) => account_type,
        GetOrCreateResponseKind::Created(account_type) => account_type,
    };

    // ? -----------------------------------------------------------------------
    // ? Fetch default account
    // ? -----------------------------------------------------------------------

    let default_subscription_account =
        match get_or_create_default_subscription_account(
            role,
            account_type,
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
                guest_role: Parent::Id(role),
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
