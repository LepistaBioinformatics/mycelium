use crate::{
    domain::{
        actors::ActorName,
        dtos::{
            account::VerboseStatus, 
            account_type::AccountTypeV2, 
            email::Email, 
            guest_user::GuestUser, 
            message::Message, 
            native_error_codes::NativeErrorCodes, 
            profile::Profile
        },
        entities::{
            AccountFetching, 
            GuestRoleFetching, 
            GuestUserRegistration, 
            MessageSending
        },
    }, 
    models::AccountLifeCycle, settings::TEMPLATES
};

use chrono::Local;
use futures::future;
use mycelium_base::{
    dtos::Parent,
    entities::{FetchResponseKind, GetOrCreateResponseKind},
    utils::errors::{use_case_err, MappedErrors},
};
use tera::Context;
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
    platform_url: Option<String>,
    life_cycle_settings: AccountLifeCycle,
    account_fetching_repo: Box<&dyn AccountFetching>,
    guest_role_fetching_repo: Box<&dyn GuestRoleFetching>,
    guest_user_registration_repo: Box<&dyn GuestUserRegistration>,
    message_sending_repo: Box<&dyn MessageSending>,
) -> Result<GetOrCreateResponseKind<GuestUser>, MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Check if the current account has sufficient privileges
    // ? -----------------------------------------------------------------------

    let related_accounts = profile
        .on_tenant(tenant_id)
        .get_related_account_with_default_write_or_error(vec![
            ActorName::TenantOwner.to_string(),
            ActorName::TenantManager.to_string(),
            ActorName::SubscriptionManager.to_string(),
        ])?;

    // ? -----------------------------------------------------------------------
    // ? Guarantee needed information to evaluate guesting
    //
    // Check if the target account is a subscription account or a standard role
    // associated account. Only these accounts can receive guesting. Already
    // check the role_id to be a guest role is valid and exists.
    //
    // ? -----------------------------------------------------------------------

    let (target_account_response, target_role_response) = future::join(
        account_fetching_repo.get(target_account_id, related_accounts),
        guest_role_fetching_repo.get(role_id),
    )
    .await;

    let target_account = match target_account_response? {
        FetchResponseKind::NotFound(id) => {
            return use_case_err(format!(
                "Target account not found: {:?}",
                id.unwrap()
            ))
            .with_code(NativeErrorCodes::MYC00013)
            .as_error()
        }
        FetchResponseKind::Found(account) => match account.account_type {
            AccountTypeV2::Subscription { .. } | 
            AccountTypeV2::StandardRoleAssociated { .. } => account,
            _ => {
                return use_case_err(
                    "Invalid account. Only subscription accounts should receive guesting."
                )
                .as_error()
            }
        },
    };

    if let Some(status) = target_account.verbose_status {
        if status != VerboseStatus::Verified {
            return use_case_err("Invalid account status. Only active accounts should receive guesting.")
                .as_error();
        }
    } else {
        return use_case_err("Unable to check account status for guesting. Account is maybe inactive.")
            .as_error();
    }

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

    let guest_user = match guest_user_registration_repo
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
        .await {
            Ok(res) => res,
            Err(err) => {
                return use_case_err(format!("Unable to create guest user: {err}"))
                    .with_code(NativeErrorCodes::MYC00017)
                    .with_exp_true()
                    .as_error()
            }
        };

    // ? -----------------------------------------------------------------------
    // ? Build notification message
    // ? -----------------------------------------------------------------------

    let mut context = Context::new();
    context.insert("account_name", &target_account.name.to_uppercase());
    context.insert("role_name", &target_role.name.to_uppercase());

    if let Some(description) = target_role.description {
        context.insert("role_description", &description);
    }

    context.insert("role_description", &target_role.name);
    context.insert(
        "role_permissions", 
        &target_role
            .permission
            .to_string()
    );

    context.insert(
        "support_email",
        &life_cycle_settings.support_email.get_or_error()?,
    );

    if let Some(url) = platform_url {
        context.insert("platform_url", &url);
    }

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

    if let Err(err) = message_sending_repo
        .send(Message {
            from: Email::from_string(
                life_cycle_settings.noreply_email.get_or_error()?,
            )?,
            to: email,
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
        return use_case_err(format!("Unable to send email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error()
    };

    // ? -----------------------------------------------------------------------
    // ? Send the guesting response
    // ? -----------------------------------------------------------------------

    Ok(guest_user)
}
