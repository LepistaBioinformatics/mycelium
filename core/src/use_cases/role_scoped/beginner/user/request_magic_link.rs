use crate::{
    domain::{
        dtos::{
            email::Email,
            native_error_codes::NativeErrorCodes,
            token::{MagicLinkTokenMeta, MultiTypeMeta},
        },
        entities::{LocalMessageWrite, TenantFetching, TokenRegistration},
    },
    models::AccountLifeCycle,
    use_cases::support::dispatch_notification,
};

use chrono::Local;
use mycelium_base::{
    entities::CreateResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

#[tracing::instrument(name = "request_magic_link", skip_all)]
pub async fn request_magic_link(
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    token_registration_repo: Box<&dyn TokenRegistration>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<(), MappedErrors> {
    // ? -----------------------------------------------------------------------
    // ? Generate magic link token meta (UUID + 6-digit code)
    // ? -----------------------------------------------------------------------

    let meta = MagicLinkTokenMeta::new(email.clone());

    // ? -----------------------------------------------------------------------
    // ? Persist the token with a 15-minute TTL
    // ? -----------------------------------------------------------------------

    let token = match token_registration_repo
        .create_magic_link_token(
            meta.clone(),
            Local::now()
                + chrono::Duration::seconds(
                    life_cycle_settings
                        .token_expiration
                        .async_get_or_error()
                        .await?,
                ),
        )
        .await
    {
        Ok(res) => match res {
            CreateResponseKind::Created(token) => token,
            CreateResponseKind::NotCreated(_, msg) => {
                return use_case_err(msg).as_error()
            }
        },
        Err(err) => return Err(err),
    };

    let token_meta = match token.meta {
        MultiTypeMeta::MagicLink(m) => m,
        _ => return use_case_err("Invalid token type").as_error(),
    };

    // ? -----------------------------------------------------------------------
    // ? Build the display URL embedded in the email link
    // ? -----------------------------------------------------------------------

    let display_token = match &token_meta.token {
        Some(t) => t.clone(),
        None => {
            return use_case_err(
                "Unexpected error: magic link token is None after creation",
            )
            .as_error()
        }
    };

    let domain_url = life_cycle_settings
        .domain_url
        .clone()
        .ok_or_else(|| {
            use_case_err("domain_url is not configured in AccountLifeCycle")
                .with_code(NativeErrorCodes::MYC00010)
        })?
        .async_get_or_error()
        .await?;

    // Percent-encode '@' in the email for use in URL query parameter
    let encoded_email = email.email().replace('@', "%40");

    let display_url = format!(
        "{}/_adm/beginners/users/magic-link/display?token={}&email={}",
        domain_url.trim_end_matches('/'),
        display_token,
        encoded_email,
    );

    // ? -----------------------------------------------------------------------
    // ? Dispatch magic link email
    // ? -----------------------------------------------------------------------

    if let Err(err) = dispatch_notification(
        vec![("magic_link_url", display_url)],
        "email/magic-link-request",
        life_cycle_settings,
        email,
        None,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
    {
        return use_case_err(format!("Unable to send magic link email: {err}"))
            .with_code(NativeErrorCodes::MYC00010)
            .as_error();
    };

    Ok(())
}
