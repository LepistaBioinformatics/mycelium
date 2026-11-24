use myc_core::{
    domain::dtos::{
        service::Service,
        service_notification_config::ServiceNotificationConfig,
    },
    models::{ServiceNotificationsConfig, ServiceTemplates},
};
use mycelium_base::utils::errors::{execution_err, MappedErrors};

/// Locale every declared service template must exist for, and the one used
/// when the preferred locale of a send has no template.
pub(crate) const SERVICE_TEMPLATES_DEFAULT_LOCALE: &str = "en-us";

/// Load and validate the service templates before the server starts
///
/// Returns `None` when no service declares a `notifications` block: such a
/// gateway boots exactly as before and needs no `templatesDir`. Otherwise the
/// templates are loaded from `templatesDir`, and a malformed template or a
/// declared template missing for the default locale is an error, which the
/// caller turns into a failed boot.
pub(crate) fn load_service_templates(
    services: &[Service],
    notifications_config: &ServiceNotificationsConfig,
) -> Result<Option<ServiceTemplates>, MappedErrors> {
    let notifications: Vec<ServiceNotificationConfig> = services
        .iter()
        .filter_map(|service| service.notifications.to_owned())
        .collect();

    if notifications.is_empty() {
        return Ok(None);
    }

    let Some(templates_dir) = notifications_config.templates_dir.as_ref()
    else {
        return execution_err(
            "core.serviceNotifications.templatesDir is required when a \
             service enables notifications",
        )
        .as_error();
    };

    let templates = ServiceTemplates::load(
        templates_dir,
        SERVICE_TEMPLATES_DEFAULT_LOCALE,
    )?;

    templates.validate(&notifications)?;

    Ok(Some(templates))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    struct TemplatesDir(PathBuf);

    impl TemplatesDir {
        fn with_files(files: &[&str]) -> Self {
            let root = std::env::temp_dir()
                .join(format!("myc-boot-service-templates-{}", Uuid::new_v4()));

            for file in files {
                let path = root.join(file);
                fs::create_dir_all(path.parent().unwrap()).unwrap();
                fs::write(path, "Hello {{ name }}").unwrap();
            }

            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn config(&self) -> ServiceNotificationsConfig {
            ServiceNotificationsConfig {
                templates_dir: Some(self.0.to_owned()),
                ..Default::default()
            }
        }
    }

    impl Drop for TemplatesDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn service(notifications: bool) -> Service {
        let mut service = serde_json::json!({
            "name": "service-01",
            "host": "localhost:8080",
            "protocol": "http",
            "routes": [],
            "healthCheckPath": "/health",
        });

        if notifications {
            service["notifications"] = serde_json::json!({
                "senderTag": "service-01",
                "displayName": "Service 01",
                "issuerRoles": ["service-01-admin"],
                "recipientPolicy": "owner",
                "templates": ["welcome"],
            });
        }

        serde_json::from_value(service).unwrap()
    }

    #[test]
    fn no_notifications_needs_no_templates_dir() {
        let loaded = load_service_templates(
            &[service(false)],
            &ServiceNotificationsConfig::default(),
        )
        .unwrap();

        assert!(loaded.is_none());
    }

    #[test]
    fn declared_templates_load() {
        let dir = TemplatesDir::with_files(&[
            "service-01/en-us/welcome.jinja",
            "service-01/en-us/welcome.subject",
        ]);

        let loaded =
            load_service_templates(&[service(true)], &dir.config()).unwrap();

        assert!(loaded.is_some());
    }

    #[test]
    fn a_missing_declared_template_fails_the_boot() {
        let dir = TemplatesDir::with_files(&["service-01/en-us/welcome.jinja"]);

        assert!(
            load_service_templates(&[service(true)], &dir.config()).is_err()
        );
    }

    #[test]
    fn a_template_only_in_another_locale_fails_the_boot() {
        let dir = TemplatesDir::with_files(&[
            "service-01/pt-br/welcome.jinja",
            "service-01/pt-br/welcome.subject",
        ]);

        assert!(
            load_service_templates(&[service(true)], &dir.config()).is_err()
        );
    }

    #[test]
    fn a_missing_templates_dir_fails_the_boot() {
        let config = ServiceNotificationsConfig {
            templates_dir: Some(std::env::temp_dir().join(format!(
                "myc-absent-service-templates-{}",
                Uuid::new_v4()
            ))),
            ..Default::default()
        };

        assert!(load_service_templates(&[service(true)], &config).is_err());
        assert!(load_service_templates(
            &[service(true)],
            &ServiceNotificationsConfig::default()
        )
        .is_err());
    }
}
