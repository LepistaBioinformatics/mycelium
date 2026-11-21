use crate::domain::dtos::native_error_codes::NativeErrorCodes;

use mycelium_base::utils::errors::{use_case_err, MappedErrors};

const SENDER_TAG_MAX_LENGTH: usize = 32;
const LOCALE_MAX_LENGTH: usize = 16;
const TEMPLATE_NAME_MAX_LENGTH: usize = 64;

/// Location of a service template: `<senderTag>/<locale>/<name>`
///
/// Every component is checked against a closed alphabet before it is joined,
/// so no component can carry `/`, `\`, `.` or any other character able to
/// leave the service directory. An invalid component is MYC00038.
///
/// - sender tag: `^[a-z0-9-]{1,32}$`
/// - locale: `^[A-Za-z0-9_-]{1,16}$`
/// - template name: `^[a-z0-9-_]{1,64}$`
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServiceTemplatePath {
    sender_tag: String,
    locale: String,
    name: String,
}

impl ServiceTemplatePath {
    pub fn new(
        sender_tag: &str,
        locale: &str,
        name: &str,
    ) -> Result<Self, MappedErrors> {
        if !is_component(sender_tag, SENDER_TAG_MAX_LENGTH, is_tag_char) {
            return invalid_component("sender tag");
        }

        if !is_component(locale, LOCALE_MAX_LENGTH, is_locale_char) {
            return invalid_component("locale");
        }

        if !is_component(name, TEMPLATE_NAME_MAX_LENGTH, is_name_char) {
            return invalid_component("template name");
        }

        Ok(Self {
            sender_tag: sender_tag.to_owned(),
            locale: locale.to_owned(),
            name: name.to_owned(),
        })
    }

    pub fn locale(&self) -> &str {
        &self.locale
    }

    /// Name of the body template inside the service templates root.
    pub fn body(&self) -> String {
        self.with_extension("jinja")
    }

    /// Name of the subject template inside the service templates root.
    pub fn subject(&self) -> String {
        self.with_extension("subject")
    }

    fn with_extension(&self, extension: &str) -> String {
        format!(
            "{}/{}/{}.{extension}",
            self.sender_tag, self.locale, self.name
        )
    }
}

fn is_component(
    value: &str,
    max_length: usize,
    is_allowed: fn(char) -> bool,
) -> bool {
    !value.is_empty()
        && value.len() <= max_length
        && value.chars().all(is_allowed)
}

fn is_tag_char(character: char) -> bool {
    character.is_ascii_lowercase()
        || character.is_ascii_digit()
        || character == '-'
}

fn is_name_char(character: char) -> bool {
    is_tag_char(character) || character == '_'
}

fn is_locale_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || character == '-' || character == '_'
}

fn invalid_component<T>(component: &str) -> Result<T, MappedErrors> {
    use_case_err(format!("Invalid service template {component}"))
        .with_code(NativeErrorCodes::MYC00038)
        .with_exp_true()
        .as_error()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TRAVERSALS: [&str; 9] = [
        "../secret",
        "..",
        ".",
        "/etc/passwd",
        "a/b",
        "a\\b",
        "a.b",
        "a\0b",
        "",
    ];

    #[test]
    fn builds_body_and_subject_names() {
        let path =
            ServiceTemplatePath::new("service-01", "en-us", "invoice_ready-2")
                .unwrap();

        assert_eq!(path.body(), "service-01/en-us/invoice_ready-2.jinja");
        assert_eq!(path.subject(), "service-01/en-us/invoice_ready-2.subject");
        assert_eq!(path.locale(), "en-us");
    }

    #[test]
    fn traversal_in_the_template_name_is_rejected() {
        for name in TRAVERSALS {
            let result = ServiceTemplatePath::new("service-01", "en-us", name);

            assert!(result.is_err(), "{name:?}");
        }
    }

    #[test]
    fn traversal_in_the_locale_is_rejected() {
        for locale in TRAVERSALS {
            let result =
                ServiceTemplatePath::new("service-01", locale, "invoice");

            assert!(result.is_err(), "{locale:?}");
        }
    }

    #[test]
    fn traversal_in_the_sender_tag_is_rejected() {
        for sender_tag in TRAVERSALS {
            let result =
                ServiceTemplatePath::new(sender_tag, "en-us", "invoice");

            assert!(result.is_err(), "{sender_tag:?}");
        }
    }

    #[test]
    fn length_limits_are_enforced() {
        let long_tag = "a".repeat(33);
        let long_locale = "a".repeat(17);
        let long_name = "a".repeat(65);

        assert!(ServiceTemplatePath::new(&long_tag, "en-us", "n").is_err());
        assert!(ServiceTemplatePath::new("t", &long_locale, "n").is_err());
        assert!(ServiceTemplatePath::new("t", "en-us", &long_name).is_err());
        assert!(ServiceTemplatePath::new("t", "en-us", &"a".repeat(64)).is_ok());
    }

    #[test]
    fn uppercase_is_rejected_in_tag_and_name() {
        assert!(ServiceTemplatePath::new("Service", "en-us", "n").is_err());
        assert!(ServiceTemplatePath::new("t", "en-us", "Invoice").is_err());
        assert!(ServiceTemplatePath::new("t", "pt-BR", "n").is_ok());
    }

    #[test]
    fn invalid_component_carries_the_template_error_code() {
        let error = ServiceTemplatePath::new("t", "en-us", "../n").unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00038]));
    }
}
