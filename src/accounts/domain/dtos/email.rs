use crate::domain::utils::errors::MappedErrors;

use regex::Regex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EmailDTO {
    pub username: String,
    pub domain: String,
}

impl EmailDTO {
    pub fn from_str(email: String) -> Result<EmailDTO, MappedErrors> {
        let re = Regex::new(
            r"^([a-z0-9_+]([a-z0-9_+.]*[a-z0-9_+])?)@([a-z0-9]+([\-\.]{1}[a-z0-9]+)*\.[a-z]{2,6})"
        ).unwrap();

        let cap = re.captures(email.as_str()).unwrap();

        let username = match cap.get(1) {
            None => {
                return Err(MappedErrors::new(
                    "".to_string(),
                    Some(true),
                    None,
                ));
            }
            Some(val) => val.as_str().to_string(),
        };

        let domain = match cap.get(2) {
            None => {
                return Err(MappedErrors::new(
                    "".to_string(),
                    Some(true),
                    None,
                ));
            }
            Some(val) => val.as_str().to_string(),
        };

        Ok(EmailDTO { username, domain })
    }
}
