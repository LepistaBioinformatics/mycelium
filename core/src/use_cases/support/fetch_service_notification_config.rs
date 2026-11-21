use crate::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        service_notification_config::ServiceNotificationConfig,
    },
    entities::ServiceRead,
};

use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{use_case_err, MappedErrors},
};

/// Fetch the notification settings of the service holding a sender tag
///
/// Read from the current services list on every call, so a service whose
/// `notifications` block was removed since a string was issued is rejected
/// with MYC00036, as is an unknown tag.
#[tracing::instrument(name = "fetch_service_notification_config", skip_all)]
pub(crate) async fn fetch_service_notification_config(
    sender_tag: &str,
    service_read_repo: Box<&dyn ServiceRead>,
) -> Result<ServiceNotificationConfig, MappedErrors> {
    let services =
        match service_read_repo.list_services(None, None, None).await? {
            FetchManyResponseKind::Found(services) => services,
            FetchManyResponseKind::FoundPaginated { records, .. } => records,
            FetchManyResponseKind::NotFound => vec![],
        };

    let service_settings = services
        .into_iter()
        .filter_map(|service| service.notifications)
        .find(|settings| settings.sender_tag == sender_tag);

    let Some(service_settings) = service_settings else {
        return use_case_err("Service notifications not enabled")
            .with_code(NativeErrorCodes::MYC00036)
            .with_exp_true()
            .as_error();
    };

    Ok(service_settings)
}
