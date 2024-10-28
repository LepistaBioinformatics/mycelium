use crate::{
    domain::{
        dtos::{
            account::Account, email::Email, guest_user::GuestUser,
            message::Message, native_error_codes::NativeErrorCodes, user::User,
        },
        entities::{
            AccountRegistration, GuestRoleFetching, GuestUserRegistration,
            MessageSending,
        },
    },
    models::AccountLifeCycle,
    settings::TEMPLATES,
    use_cases::roles::shared::account::get_or_create_role_related_account,
};

use chrono::Local;
use futures::future;
use mycelium_base::{
    dtos::{Children, Parent},
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;
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
    life_cycle_settings: AccountLifeCycle,
    account_registration_repo: Box<&dyn AccountRegistration>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    message_sending_repo: Box<&dyn MessageSending>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Guarantee needed information to evaluate guesting
    //
    // Check if the target account is a subscription account or a standard role
    // associated account. Only these accounts can receive guesting. Already
    // check the role_id to be a guest role is valid and exists.
    //
    // ? -----------------------------------------------------------------------

    let (target_account_response, target_role_response) = future::join(
        get_or_create_role_related_account(
            tenant_id,
            role_id,
            account_registration_repo,
        ),
        guest_role_fetching_repo.get(role_id),
    )
    .await;

    let default_subscription_account = match target_account_response? {
        GetOrCreateResponseKind::NotCreated(account, _) => account,
        GetOrCreateResponseKind::Created(account) => account,
    };

    let target_role = match target_role_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Guest role not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00012)
            .as_error()
        }
        FetchResponseKind::Found(role) => role,
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
    // ? Build notification message
    // ? -----------------------------------------------------------------------

    let mut context = Context::new();
    context.insert(
        "account_name",
        &default_subscription_account.name.to_uppercase(),
    );
    if let Some(description) = target_role.description {
        context.insert("role_description", &description);
    }

    context.insert("role_name", &target_role.name.to_uppercase());
    context.insert("role_description", &target_role.name);
    context.insert("role_permissions", &target_role.permission.to_string());

    context.insert(
        "support_email",
        &life_cycle_settings.support_email.get_or_error()?,
    );

    let email_template = match TEMPLATES
        .render("email/guest-to-subscription-account.jinja", &context)
    {
        Ok(res) => res,
        Err(err) => {
            return use_case_err(format!(
                "Unable to render email template: {err}"
            ))
            .as_error();
        }
    };

    // ? -----------------------------------------------------------------------
    // ? Notify guest user
    // ? -----------------------------------------------------------------------

    match message_sending_repo
        .send(Message {
            from: Email::from_string(
                life_cycle_settings.noreply_email.get_or_error()?,
            )?,
            to: guest_email,
            cc: None,
            subject: String::from(
                "[Guest to Account] You have been invited to collaborate",
            ),
            message_head: None,
            message_body: email_template,
            message_footer: None,
        })
        .await
    {
        Err(err) => {
            return use_case_err(format!("Unable to send email: {err}"))
                .with_code(NativeErrorCodes::MYC00010)
                .as_error()
        }
        Ok(res) => {
            info!("Guesting to default account successfully done: {:?}", res)
        }
    };

    Ok(())
}
