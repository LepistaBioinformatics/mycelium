use mycelium_base::utils::errors::{invalid_arg_err, MappedErrors};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Email {
    pub username: String,
    pub domain: String,
}

impl Email {
    pub fn from_string(email: String) -> Result<Email, MappedErrors> {
        let re = Regex::new(
            r"^([a-zA-Z0-9_\-+]([a-zA-Z0-9_\-+.]*[a-zA-Z0-9_+])?)@([a-zA-Z0-9.-]+\.[a-zA-Z]{1,})"
        ).unwrap();

        let cap = match re.captures(email.as_str()) {
            None => {
                return invalid_arg_err(format!(
                    "Invalid Email format: {:?}",
                    email.to_owned()
                ))
                .as_error();
            }
            Some(res) => res,
        };

        let username = match cap.get(1) {
            None => {
                return invalid_arg_err("Invalid Email username.".to_string())
                    .as_error();
            }
            Some(val) => val.as_str().to_string(),
        };

        let domain = match cap.get(3) {
            None => {
                return invalid_arg_err("Invalid Email domain.".to_string())
                    .as_error();
            }
            Some(val) => val.as_str().to_string(),
        };

        Ok(Email { username, domain })
    }

    pub fn email(&self) -> String {
        format!(
            "{}@{}",
            self.username.to_lowercase(),
            self.domain.to_lowercase()
        )
    }

    /// Get redacted email
    ///
    /// Return only the first and last letters o the email.username and the
    /// domain
    ///
    pub fn redacted_email(&self) -> String {
        Self::redact_email(&self.email())
    }

    /// Effectively redact the email
    ///
    /// This is a static function used to expose the functionality
    ///
    pub fn redact_email(email: &str) -> String {
        let email = match Email::from_string(email.to_string()) {
            Ok(email) => email,
            Err(e) => {
                tracing::warn!("Invalid Email format: {:?}", e);
                return email.to_owned();
            }
        };

        let binding = email.username.to_lowercase();
        let username = binding.chars();
        let domain = email.domain.to_lowercase();

        let username_redacted = format!(
            "{}{}{}",
            username.to_owned().next().unwrap(),
            "*".repeat(3),
            username.last().unwrap()
        );

        format!("{}@{}", username_redacted, domain)
    }
}

impl ToString for Email {
    fn to_string(&self) -> String {
        self.email()
    }
}

// ? --------------------------------------------------------------------------
// ? TESTS
// ? --------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_email_works() {
        let email_string = "sgelias@outlook.com".to_string();

        let email = Email::from_string(email_string.to_owned()).unwrap();

        assert_eq!(email.username, "sgelias".to_string());
        assert_eq!(email.domain, "outlook.com".to_string());
    }

    #[test]
    fn test_get_email_works() {
        for (is_valid, email_string) in vec![
            (true, "mycelium-default-users@biotrop.com.br".to_string()),
            (true, "myceliumDefaultUsers@biotrop.com.br".to_string()),
            (true, "mycelium-default-users@biotrop.com".to_string()),
            (true, "myceliumDefaultUsers@biotrop.com".to_string()),
            (false, "mycelium-default-users@biotrop".to_string()),
            (false, "myceliumDefaultUsers@biotrop".to_string()),
        ] {
            let email = Email::from_string(email_string.to_owned());

            if is_valid {
                assert_eq!(email.unwrap().email(), email_string.to_lowercase());
            } else {
                assert!(email.is_err());
            }
        }
    }
}
