use agrobase::utils::errors::{invalid_arg_err, MappedErrors};
use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDTO {
    pub username: String,
    pub domain: String,
}

impl EmailDTO {
    pub fn from_string(email: String) -> Result<EmailDTO, MappedErrors> {
        let re = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})"
        ).unwrap();

        let cap = re.captures(email.as_str()).unwrap();

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

        Ok(EmailDTO { username, domain })
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

        let email = EmailDTO::from_string(email_string.to_owned()).unwrap();

        assert_eq!(email.username, "sgelias".to_string());
        assert_eq!(email.domain, "outlook.com".to_string());
    }

    #[test]
    fn test_get_email_works() {
        let email_string = "sgelias@outlook.com".to_string();

        let email = EmailDTO::from_string(email_string.to_owned()).unwrap();

        assert_eq!(email.get_email(), email_string);
    }
}
