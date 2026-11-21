use super::{
    rendered_service_email::RenderedServiceEmail,
    service_template_path::ServiceTemplatePath,
};
use crate::domain::dtos::{
    native_error_codes::NativeErrorCodes,
    service_notification_config::ServiceNotificationConfig,
};

use mycelium_base::utils::errors::{execution_err, use_case_err, MappedErrors};
use regex::Regex;
use serde_json::Value;
use std::{error::Error, path::Path};
use tera::{Context, Tera};

/// Service-owned email templates, isolated from the gateway `TEMPLATES`
///
/// Loaded eagerly from `templatesDir/<senderTag>/<locale>/<name>.jinja` and
/// `.subject`. A malformed template fails the load, so the operator learns
/// about it at boot and the gateway lifecycle emails are never affected.
/// Autoescape is on for both extensions.
#[derive(Debug)]
pub struct ServiceTemplates {
    engine: Tera,
    default_locale: String,
}

impl ServiceTemplates {
    /// Load every template under `templates_dir`. Any error is fatal.
    pub fn load(
        templates_dir: &Path,
        default_locale: &str,
    ) -> Result<Self, MappedErrors> {
        let mut engine = load_engine(templates_dir)?;

        engine.autoescape_on(vec![".jinja", ".subject"]);

        Ok(Self {
            engine,
            default_locale: default_locale.to_owned(),
        })
    }

    /// Check that every declared template exists for the default locale.
    pub fn validate(
        &self,
        services: &[ServiceNotificationConfig],
    ) -> Result<(), MappedErrors> {
        services
            .iter()
            .try_for_each(|service| self.validate_service(service))
    }

    fn validate_service(
        &self,
        service: &ServiceNotificationConfig,
    ) -> Result<(), MappedErrors> {
        service.templates.iter().try_for_each(|name| {
            self.validate_template(&service.sender_tag, name)
        })
    }

    fn validate_template(
        &self,
        sender_tag: &str,
        name: &str,
    ) -> Result<(), MappedErrors> {
        let path =
            ServiceTemplatePath::new(sender_tag, &self.default_locale, name)?;

        if self.contains(&path) {
            return Ok(());
        }

        execution_err(format!(
            "Service template declared but not found: {} and {}",
            path.body(),
            path.subject()
        ))
        .as_error()
    }

    /// Render the subject and body of a service template
    ///
    /// The requested locale is used when the service has the template for
    /// it, the default locale otherwise. A missing template, a missing
    /// variable or an invalid path component is MYC00038.
    pub fn render(
        &self,
        sender_tag: &str,
        locale: &str,
        name: &str,
        variables: Value,
    ) -> Result<RenderedServiceEmail, MappedErrors> {
        let path = self.resolve_path(sender_tag, locale, name)?;

        let context = Context::from_value(variables).map_err(|_| {
            template_err("Invalid template variables".to_string())
        })?;

        let body = self.render_file(&path.body(), &context)?;
        let subject = self.render_file(&path.subject(), &context)?;

        Ok(RenderedServiceEmail {
            subject: single_line_subject(subject)?,
            body,
            locale: path.locale().to_owned(),
        })
    }

    fn resolve_path(
        &self,
        sender_tag: &str,
        locale: &str,
        name: &str,
    ) -> Result<ServiceTemplatePath, MappedErrors> {
        let requested = ServiceTemplatePath::new(sender_tag, locale, name)?;

        if self.contains(&requested) {
            return Ok(requested);
        }

        let fallback =
            ServiceTemplatePath::new(sender_tag, &self.default_locale, name)?;

        if self.contains(&fallback) {
            return Ok(fallback);
        }

        Err(template_err(format!("Service template not found: {name}")))
    }

    fn contains(&self, path: &ServiceTemplatePath) -> bool {
        self.engine.get_template(&path.body()).is_ok()
            && self.engine.get_template(&path.subject()).is_ok()
    }

    fn render_file(
        &self,
        file: &str,
        context: &Context,
    ) -> Result<String, MappedErrors> {
        self.engine
            .render(file, context)
            .map_err(|err| template_err(describe_render_failure(&err)))
    }
}

/// The reason a render failed, without any value taken from the request
///
/// Render errors reach the error log, and Tera puts the offending value in
/// some messages (a filter that receives the wrong type, for instance).
/// Only the name of a missing variable is reported, and only when the error
/// is exactly Tera's missing-variable message: the name comes from the
/// template, and matching the whole message keeps a value that merely
/// contains similar words from passing through.
///
fn describe_render_failure(err: &tera::Error) -> String {
    let Some(name) = missing_variable_name(err) else {
        return "Unable to render service template: a variable has an \
                unexpected value"
            .to_string();
    };

    format!("Unable to render service template: variable `{name}` is missing")
}

/// The variable name of Tera's "Variable `x` not found in context" error
fn missing_variable_name(err: &tera::Error) -> Option<String> {
    let source = err.source()?.to_string();
    let pattern = Regex::new(MISSING_VARIABLE_PATTERN).ok()?;
    let captures = pattern.captures(&source)?;

    Some(captures.get(1)?.as_str().to_owned())
}

fn load_engine(templates_dir: &Path) -> Result<Tera, MappedErrors> {
    if !templates_dir.is_dir() {
        return execution_err(format!(
            "Service templates directory not found: {}",
            templates_dir.display()
        ))
        .as_error();
    }

    Tera::new(&format!("{}/**/*", templates_dir.display())).map_err(|err| {
        execution_err(format!(
            "Unable to load service templates: {}",
            describe(&err)
        ))
    })
}

/// Tera's whole message for a variable missing from the render context;
/// the name is limited to the characters a template identifier can use
const MISSING_VARIABLE_PATTERN: &str = r"^Variable `([A-Za-z0-9_.]{1,64})` not found in context while rendering '[^'`]*'$";

/// A subject becomes a mail header: it must stay on a single line.
fn single_line_subject(subject: String) -> Result<String, MappedErrors> {
    let subject = subject.trim();

    if subject.chars().any(char::is_control) {
        return Err(template_err(
            "Rendered subject carries a line break or control character"
                .to_string(),
        ));
    }

    Ok(subject.to_owned())
}

/// Tera keeps the useful part (e.g. the missing variable) in the source.
fn describe(err: &tera::Error) -> String {
    let Some(source) = err.source() else {
        return err.to_string();
    };

    format!("{err}: {source}")
}

fn template_err(message: String) -> MappedErrors {
    use_case_err(message)
        .with_code(NativeErrorCodes::MYC00038)
        .with_exp_true()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::service_notification_config::RecipientPolicy;
    use serde_json::json;
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    struct TemplatesDir(PathBuf);

    impl TemplatesDir {
        fn new() -> Self {
            let root = std::env::temp_dir()
                .join(format!("myc-service-templates-{}", Uuid::new_v4()));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }

        fn write(&self, file: &str, content: &str) -> &Self {
            let path = self.0.join(file);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, content).unwrap();
            self
        }

        fn with_invoice(self, locale: &str, greeting: &str) -> Self {
            let prefix = format!("service-01/{locale}/invoice-ready");
            self.write(
                &format!("{prefix}.jinja"),
                &format!("<p>{greeting} {{{{ customer }}}}</p>"),
            )
            .write(
                &format!("{prefix}.subject"),
                &format!("{greeting} {{{{ customer }}}}\n"),
            );
            self
        }

        fn load(&self) -> Result<ServiceTemplates, MappedErrors> {
            ServiceTemplates::load(&self.0, "en-us")
        }
    }

    impl Drop for TemplatesDir {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn service(templates: Vec<&str>) -> ServiceNotificationConfig {
        ServiceNotificationConfig {
            sender_tag: "service-01".to_string(),
            display_name: "Service 01".to_string(),
            issuer_roles: vec!["service-01-admin".to_string()],
            issuer_tenants: vec![],
            recipient_policy: RecipientPolicy::Owner,
            allowed_domains: vec![],
            templates: templates.into_iter().map(str::to_string).collect(),
            quota_per_hour: 100,
            max_recipients_per_message: 10,
        }
    }

    fn assert_template_error<T: std::fmt::Debug>(
        result: Result<T, MappedErrors>,
    ) {
        let error = result.unwrap_err();
        assert!(error.is_in(vec![NativeErrorCodes::MYC00038]), "{error}");
    }

    #[test]
    fn renders_subject_and_body() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        let rendered = templates
            .render(
                "service-01",
                "en-us",
                "invoice-ready",
                json!({"customer": "Ana"}),
            )
            .unwrap();

        assert_eq!(rendered.body, "<p>Hello Ana</p>");
        assert_eq!(rendered.subject, "Hello Ana");
        assert_eq!(rendered.locale, "en-us");
    }

    #[test]
    fn html_in_a_variable_is_escaped_in_body_and_subject() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        let rendered = templates
            .render(
                "service-01",
                "en-us",
                "invoice-ready",
                json!({"customer": "<script>alert(1)</script>"}),
            )
            .unwrap();

        assert!(!rendered.body.contains("<script>"), "{}", rendered.body);
        assert!(rendered.body.contains("&lt;script&gt;"));
        assert!(!rendered.subject.contains("<script>"));
        assert!(rendered.subject.contains("&lt;script&gt;"));
    }

    #[test]
    fn line_break_in_a_subject_variable_is_rejected() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        for customer in ["Ana\r\nBcc: attacker@example.com", "Ana\nX", "A\rB"] {
            assert_template_error(templates.render(
                "service-01",
                "en-us",
                "invoice-ready",
                json!({ "customer": customer }),
            ));
        }
    }

    #[test]
    fn requested_locale_is_used_when_present() {
        let dir = TemplatesDir::new()
            .with_invoice("en-us", "Hello")
            .with_invoice("pt-br", "Olá");
        let templates = dir.load().unwrap();

        let rendered = templates
            .render(
                "service-01",
                "pt-br",
                "invoice-ready",
                json!({"customer": "Ana"}),
            )
            .unwrap();

        assert_eq!(rendered.subject, "Olá Ana");
        assert_eq!(rendered.locale, "pt-br");
    }

    #[test]
    fn unknown_locale_falls_back_to_the_default_one() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        let rendered = templates
            .render(
                "service-01",
                "fr-fr",
                "invoice-ready",
                json!({"customer": "Ana"}),
            )
            .unwrap();

        assert_eq!(rendered.subject, "Hello Ana");
        assert_eq!(rendered.locale, "en-us");
    }

    #[test]
    fn missing_template_is_a_template_error() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        assert_template_error(templates.render(
            "service-01",
            "en-us",
            "unknown",
            json!({}),
        ));
    }

    #[test]
    fn template_of_another_service_is_not_reachable() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        assert_template_error(templates.render(
            "service-02",
            "en-us",
            "invoice-ready",
            json!({"customer": "Ana"}),
        ));
    }

    #[test]
    fn missing_variable_is_a_template_error() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        let result =
            templates.render("service-01", "en-us", "invoice-ready", json!({}));

        let error = result.unwrap_err();
        assert!(error.is_in(vec![NativeErrorCodes::MYC00038]), "{error}");
        assert!(error.to_string().contains("customer"), "{error}");
    }

    #[test]
    fn render_error_never_carries_a_variable_value() {
        let dir = TemplatesDir::new();
        dir.write(
            "service-01/en-us/invoice-ready.jinja",
            "{{ amount | round }}",
        )
        .write("service-01/en-us/invoice-ready.subject", "Invoice\n");
        let templates = dir.load().unwrap();

        let error = templates
            .render(
                "service-01",
                "en-us",
                "invoice-ready",
                json!({"amount": "private-value-9f3"}),
            )
            .unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00038]), "{error}");
        let rendered = format!("{error} {error:?}");
        assert!(!rendered.contains("private-value-9f3"), "{rendered}");
        assert!(rendered.contains("unexpected value"), "{rendered}");
    }

    #[test]
    fn value_imitating_the_missing_variable_message_is_not_echoed() {
        let dir = TemplatesDir::new();
        dir.write(
            "service-01/en-us/invoice-ready.jinja",
            "{{ amount | round }}",
        )
        .write("service-01/en-us/invoice-ready.subject", "Invoice\n");
        let templates = dir.load().unwrap();
        let value = "Variable `secret-7c1` not found in context while \
                     rendering 'private-value-7c1'";

        let error = templates
            .render(
                "service-01",
                "en-us",
                "invoice-ready",
                json!({ "amount": value }),
            )
            .unwrap_err();

        let rendered = format!("{error} {error:?}");
        assert!(!rendered.contains("7c1"), "{rendered}");
    }

    #[test]
    fn non_object_variables_are_a_template_error() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        assert_template_error(templates.render(
            "service-01",
            "en-us",
            "invoice-ready",
            json!(["Ana"]),
        ));
    }

    #[test]
    fn path_traversal_is_rejected_in_every_component() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        dir.write("secret.jinja", "secret")
            .write("secret.subject", "s");
        let templates = dir.load().unwrap();
        let vars = json!({"customer": "Ana"});

        assert_template_error(templates.render(
            "service-01",
            "en-us",
            "../../secret",
            vars.clone(),
        ));
        assert_template_error(templates.render(
            "service-01",
            "..",
            "secret",
            vars.clone(),
        ));
        assert_template_error(templates.render(
            "../service-01",
            "en-us",
            "invoice-ready",
            vars.clone(),
        ));
        assert_template_error(templates.render(
            "service-01",
            "en-us",
            "/etc/passwd",
            vars,
        ));
    }

    #[test]
    fn malformed_template_fails_the_load() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        dir.write("service-01/en-us/broken.jinja", "{% if %}");

        assert!(dir.load().is_err());
    }

    #[test]
    fn missing_directory_fails_the_load() {
        let missing = std::env::temp_dir()
            .join(format!("myc-service-templates-{}", Uuid::new_v4()));

        assert!(ServiceTemplates::load(&missing, "en-us").is_err());
    }

    #[test]
    fn validate_accepts_declared_templates_present_for_default_locale() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        assert!(templates
            .validate(&[service(vec!["invoice-ready"])])
            .is_ok());
    }

    #[test]
    fn validate_rejects_a_template_missing_for_the_default_locale() {
        let dir = TemplatesDir::new().with_invoice("pt-br", "Olá");
        let templates = dir.load().unwrap();

        assert!(templates
            .validate(&[service(vec!["invoice-ready"])])
            .is_err());
    }

    #[test]
    fn validate_rejects_a_template_without_its_subject() {
        let dir = TemplatesDir::new();
        dir.write("service-01/en-us/invoice-ready.jinja", "<p>Hello</p>");
        let templates = dir.load().unwrap();

        assert!(templates
            .validate(&[service(vec!["invoice-ready"])])
            .is_err());
    }

    #[test]
    fn validate_rejects_a_declared_name_with_traversal() {
        let dir = TemplatesDir::new().with_invoice("en-us", "Hello");
        let templates = dir.load().unwrap();

        assert!(templates
            .validate(&[service(vec!["../invoice-ready"])])
            .is_err());
    }
}
