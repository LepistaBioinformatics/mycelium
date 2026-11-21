use crate::{
    domain::{
        dtos::{
            native_error_codes::NativeErrorCodes, profile::Profile,
            service_notification_config::ServiceNotificationConfig,
            token::UserAccountConnectionString,
        },
        entities::{LicensedResourcesFetching, ProfileFetching},
    },
    use_cases::{
        service::profile::{fetch_profile_from_email, ProfileResponse},
        support::check_notification_issuer,
    },
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Check again, at send time, that the string creator may send
///
/// The profile is read straight from the database (never from the KV
/// cache), so a deactivated account, a revoked role or a removed tenant
/// licence stops the string on the next send. The creator must still be a
/// registered user whose account is the string `AID`, and must pass
/// `check_notification_issuer` (no staff or manager bypass). Every failure is
/// MYC00037.
#[tracing::instrument(name = "recheck_notification_issuer", skip_all)]
pub(crate) async fn recheck_notification_issuer(
    connection_string: &UserAccountConnectionString,
    config: &ServiceNotificationConfig,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<Profile, MappedErrors> {
    let tenant_id = connection_string.scope.get_tenant_id();

    let response = fetch_profile_from_email(
        connection_string.email.to_owned(),
        None,
        tenant_id,
        None,
        profile_fetching_repo,
        licensed_resources_fetching_repo,
    )
    .await?;

    let ProfileResponse::RegisteredUser(profile) = response else {
        return issuer_gone();
    };

    if !is_the_string_account(&profile, connection_string) {
        return issuer_gone();
    }

    check_notification_issuer(&profile, tenant_id, config)?;

    Ok(profile)
}

fn is_the_string_account(
    profile: &Profile,
    connection_string: &UserAccountConnectionString,
) -> bool {
    profile.acc_id == connection_string.account_id
        && connection_string.scope.get_user_account_id() == Some(profile.acc_id)
}

fn issuer_gone() -> Result<Profile, MappedErrors> {
    use_case_err("Notification string creator is no longer a valid issuer")
        .with_code(NativeErrorCodes::MYC00037)
        .with_exp_true()
        .as_error()
}
