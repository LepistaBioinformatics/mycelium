use clean_base::utils::errors::{invalid_arg_err, MappedErrors};
use regex::Regex;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, ToSchema)]
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
                return Err(invalid_arg_err(
                    format!("Invalid Email format: {:?}", email.to_owned()),
                    Some(true),
                    None,
                ));
            }
            Some(res) => res,
        };

        let username = match cap.get(1) {
            None => {
                return Err(invalid_arg_err(
                    "Invalid Email username.".to_string(),
                    Some(true),
                    None,
                ));
            }
            Some(val) => val.as_str().to_string(),
        };

        let domain = match cap.get(3) {
            None => {
                return Err(invalid_arg_err(
                    "Invalid Email domain.".to_string(),
                    Some(true),
                    None,
                ));
            }
            Some(val) => val.as_str().to_string(),
        };

        Ok(Email { username, domain })
    }

    pub fn get_email(&self) -> String {
        format!("{}@{}", self.username, self.domain)
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
                assert_eq!(email.unwrap().get_email(), email_string);
            } else {
                assert!(email.is_err());
            }
        }
    }
}
