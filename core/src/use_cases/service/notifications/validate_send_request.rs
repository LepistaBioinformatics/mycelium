use crate::domain::dtos::{
    native_error_codes::NativeErrorCodes,
    send_service_email_request::SendServiceEmailRequest,
    service_notification_config::ServiceNotificationConfig,
};

use mycelium_base::utils::errors::{use_case_err, MappedErrors};
use serde_json::{Map, Value};

/// Largest accepted `variables` object, in bytes of compact JSON.
pub const MAX_TEMPLATE_VARIABLES_BYTES: usize = 8 * 1024;

/// Check a send request against the service notification settings
///
/// - `1 <= to.len() <= maxRecipientsPerMessage`;
/// - `template` is declared in the service `templates`;
/// - every variable is a string, a number or a boolean (no object, array or
///   null);
/// - the variables, serialized as compact JSON (`serde_json::to_vec` of the
///   whole object, keys, quotes and separators included), take at most
///   `MAX_TEMPLATE_VARIABLES_BYTES` bytes.
///
/// Every violation is MYC00038. Messages never repeat the request values.
pub(crate) fn validate_send_request(
    request: &SendServiceEmailRequest,
    config: &ServiceNotificationConfig,
) -> Result<(), MappedErrors> {
    let max_recipients = config.max_recipients_per_message as usize;

    if request.to.is_empty() || request.to.len() > max_recipients {
        return invalid_content(&format!(
            "Recipients must be between 1 and {max_recipients}"
        ));
    }

    if !config.templates.contains(&request.template) {
        return invalid_content("Template not declared for this service");
    }

    validate_template_variables(&request.variables)
}

fn validate_template_variables(
    variables: &Map<String, Value>,
) -> Result<(), MappedErrors> {
    if !variables.values().all(is_scalar) {
        return invalid_content(
            "Template variables must be strings, numbers or booleans",
        );
    }

    let size = serde_json::to_vec(variables)
        .map_err(|_| invalid_content_error("Invalid template variables"))?
        .len();

    if size > MAX_TEMPLATE_VARIABLES_BYTES {
        return invalid_content(&format!(
            "Template variables exceed {MAX_TEMPLATE_VARIABLES_BYTES} bytes"
        ));
    }

    Ok(())
}

fn is_scalar(value: &Value) -> bool {
    matches!(value, Value::String(_) | Value::Number(_) | Value::Bool(_))
}

fn invalid_content(message: &str) -> Result<(), MappedErrors> {
    Err(invalid_content_error(message))
}

fn invalid_content_error(message: &str) -> MappedErrors {
    use_case_err(message)
        .with_code(NativeErrorCodes::MYC00038)
        .with_exp_true()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn config() -> ServiceNotificationConfig {
        serde_json::from_value(json!({
            "senderTag": "service-01",
            "displayName": "Service 01",
            "issuerRoles": ["service-01-admin"],
            "recipientPolicy": "any",
            "templates": ["welcome"],
            "maxRecipientsPerMessage": 2,
        }))
        .unwrap()
    }

    fn request(
        to: usize,
        template: &str,
        variables: Value,
    ) -> SendServiceEmailRequest {
        serde_json::from_value(json!({
            "to": vec!["user@example.com"; to],
            "template": template,
            "variables": variables,
        }))
        .unwrap()
    }

    fn assert_invalid(result: Result<(), MappedErrors>) {
        let error = result.unwrap_err();

        assert!(error.is_in(vec![NativeErrorCodes::MYC00038]), "{error}");
        assert!(error.expected());
    }

    /// A variables object whose compact JSON takes exactly `size` bytes.
    fn variables_of_size(size: usize) -> Value {
        // `{"v":""}` is 8 bytes of overhead.
        json!({ "v": "x".repeat(size - 8) })
    }

    #[test]
    fn accepts_a_valid_request() {
        let variables = json!({ "name": "User", "count": 3, "vip": true });

        validate_send_request(&request(2, "welcome", variables), &config())
            .unwrap();
    }

    #[test]
    fn rejects_a_recipient_count_out_of_range() {
        assert_invalid(validate_send_request(
            &request(0, "welcome", json!({})),
            &config(),
        ));
        assert_invalid(validate_send_request(
            &request(3, "welcome", json!({})),
            &config(),
        ));
    }

    #[test]
    fn rejects_an_undeclared_template() {
        assert_invalid(validate_send_request(
            &request(1, "invoice-ready", json!({})),
            &config(),
        ));
    }

    #[test]
    fn rejects_non_scalar_variables() {
        for value in [json!({ "a": 1 }), json!([1]), Value::Null] {
            assert_invalid(validate_send_request(
                &request(1, "welcome", json!({ "nested": value })),
                &config(),
            ));
        }
    }

    #[test]
    fn variables_size_boundary_is_inclusive() {
        let at_limit = variables_of_size(MAX_TEMPLATE_VARIABLES_BYTES);
        let over_limit = variables_of_size(MAX_TEMPLATE_VARIABLES_BYTES + 1);

        assert_eq!(
            serde_json::to_vec(&at_limit).unwrap().len(),
            MAX_TEMPLATE_VARIABLES_BYTES
        );

        validate_send_request(&request(1, "welcome", at_limit), &config())
            .unwrap();
        assert_invalid(validate_send_request(
            &request(1, "welcome", over_limit),
            &config(),
        ));
    }
}
