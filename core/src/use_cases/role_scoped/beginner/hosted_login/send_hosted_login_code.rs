use super::attach_hosted_login_code;
use crate::{
    domain::{
        dtos::{
            email::Email, hosted_login::HostedLoginSession,
            native_error_codes::NativeErrorCodes,
        },
        entities::{
            HostedLoginSessionFetching, HostedLoginSessionUpdating,
            LocalMessageWrite, TenantFetching,
        },
    },
    models::AccountLifeCycle,
    use_cases::support::dispatch_notification,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Attach a new code to the hosted login session and email it.
#[tracing::instrument(name = "send_hosted_login_code", skip_all)]
pub async fn send_hosted_login_code(
    session_id: String,
    csrf_token: String,
    email: Email,
    life_cycle_settings: AccountLifeCycle,
    session_fetching_repo: Box<&dyn HostedLoginSessionFetching>,
    session_updating_repo: Box<&dyn HostedLoginSessionUpdating>,
    message_sending_repo: Box<&dyn LocalMessageWrite>,
    tenant_fetching_repo: Box<&dyn TenantFetching>,
) -> Result<HostedLoginSession, MappedErrors> {
    let (session, email_code) = attach_hosted_login_code(
        session_id,
        csrf_token,
        email,
        session_fetching_repo,
        session_updating_repo,
    )
    .await?;

    if let Err(err) = dispatch_notification(
        vec![("code", email_code.code)],
        "email/hosted-login-code",
        life_cycle_settings,
        email_code.email,
        None,
        message_sending_repo,
        tenant_fetching_repo,
    )
    .await
    {
        return use_case_err(format!(
            "Unable to send hosted login code email: {err}"
        ))
        .with_code(NativeErrorCodes::MYC00010)
        .as_error();
    }

    Ok(session)
}
