use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use utoipa::ToSchema;

/// Body of a downstream service email request
///
/// Only these four fields exist, and unknown fields are rejected, so the
/// caller can never choose the sender, the reply address, the subject, raw
/// HTML or any mail header: those come from the gateway config and the
/// operator-deployed template.
#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SendServiceEmailRequest {
    /// Recipient addresses, one message each (recipients never see each
    /// other). Each entry must be a single bare address.
    pub to: Vec<String>,

    /// Name of a template declared in the service `templates` list.
    pub template: String,

    /// Flat object of strings, numbers and booleans used by the template.
    /// At most 8192 bytes once serialized as compact JSON.
    #[schema(value_type = Object)]
    pub variables: Map<String, Value>,

    /// Preferred locale. Falls back to the tenant, the instance and finally
    /// the default locale of the service templates.
    #[serde(default)]
    pub locale: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_four_fields() {
        let request: SendServiceEmailRequest =
            serde_json::from_value(serde_json::json!({
                "to": ["user@example.com"],
                "template": "welcome",
                "variables": { "name": "User" },
                "locale": "pt-br",
            }))
            .unwrap();

        assert_eq!(request.to, vec!["user@example.com".to_string()]);
        assert_eq!(request.locale.as_deref(), Some("pt-br"));
    }

    #[test]
    fn rejects_every_field_that_would_shape_the_mail() {
        for field in [
            "from", "replyTo", "subject", "body", "headers", "html", "cc",
        ] {
            let mut body = serde_json::json!({
                "to": ["user@example.com"],
                "template": "welcome",
                "variables": {},
            });
            body[field] = serde_json::json!("value");

            let result =
                serde_json::from_value::<SendServiceEmailRequest>(body);

            assert!(result.is_err(), "{field} must be rejected");
        }
    }
}
