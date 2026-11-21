use super::recheck_notification_issuer;
use crate::{
    domain::{
        dtos::{
            native_error_codes::NativeErrorCodes, profile::Profile,
            service_notification_config::ServiceNotificationConfig,
            token::UserAccountConnectionString,
        },
        entities::{LicensedResourcesFetching, ProfileFetching, ServiceRead},
    },
    use_cases::support::fetch_service_notification_config,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

/// Authorize a send, as of now, for the service bound to the string
///
/// The service comes from the string `SVC` bean (never from the request) and
/// must still enable notifications (MYC00036). The string creator is then
/// re-checked on a fresh profile (MYC00037). Returns the service settings
/// and the creator profile.
#[tracing::instrument(name = "authorize_service_sender", skip_all)]
pub(crate) async fn authorize_service_sender(
    connection_string: &UserAccountConnectionString,
    service_read_repo: Box<&dyn ServiceRead>,
    profile_fetching_repo: Box<&dyn ProfileFetching>,
    licensed_resources_fetching_repo: Box<&dyn LicensedResourcesFetching>,
) -> Result<(ServiceNotificationConfig, Profile), MappedErrors> {
    let Some(sender_tag) = connection_string.scope.get_sender_tag() else {
        return use_case_err("Service notifications not enabled")
            .with_code(NativeErrorCodes::MYC00036)
            .with_exp_true()
            .as_error();
    };

    let config =
        fetch_service_notification_config(&sender_tag, service_read_repo)
            .await?;

    let profile = recheck_notification_issuer(
        connection_string,
        &config,
        profile_fetching_repo,
        licensed_resources_fetching_repo,
    )
    .await?;

    Ok((config, profile))
}
