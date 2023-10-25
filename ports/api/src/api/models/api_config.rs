use clean_base::utils::errors::{factories::creation_err, MappedErrors};
use myc_config::{load_config_from_file, optional_config::OptionalConfig};
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsConfig {
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ApiConfig {
    pub service_ip: String,
    pub service_port: u16,
    pub allowed_origins: Vec<String>,
    pub service_workers: i32,
    pub gateway_timeout: u64,
    pub logging_level: String,
    pub routes: String,
    pub tls: OptionalConfig<TlsConfig>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TmpConfig {
    api: ApiConfig,
}

impl ApiConfig {
    pub fn from_default_config_file(
        file: PathBuf,
    ) -> Result<Self, MappedErrors> {
        if !file.exists() {
            return creation_err(format!(
                "Could not find config file: {}",
                file.to_str().unwrap()
            ))
            .as_error();
        }

        match load_config_from_file::<TmpConfig>(file) {
            Ok(config) => Ok(config.api),
            Err(err) => Err(err),
        }
    }
}

/* pub struct SvcConfig {
    pub service_ip: String,
    pub service_port: u16,
    pub allowed_origins: Vec<String>,
    pub service_workers: i32,
    pub gateway_timeout: u64,
    pub tls_cert_path: Option<String>,
    pub tls_key_path: Option<String>,
    pub token_secret_key: String,
    pub token_expiration: i64,
    pub token_hmac_secret: String,
    pub token_email_notifier: Email,
}

impl SvcConfig {
    pub fn new() -> Self {
        Self {
            service_ip: match var_os("SERVICE_IP") {
                Some(path) => path.into_string().unwrap(),
                None => String::from("0.0.0.0"),
            },
            service_port: match var_os("SERVICE_PORT") {
                Some(path) => {
                    path.into_string().unwrap().parse::<u16>().unwrap()
                }
                None => 8080,
            },
            allowed_origins: match var_os("ALLOWED_ORIGINS") {
                Some(path) => path
                    .into_string()
                    .unwrap()
                    .split(",")
                    .into_iter()
                    .map(|i| i.to_string())
                    .collect(),
                None => vec!["http://localhost:8080".to_string()],
            },
            service_workers: match var_os("SERVICE_WORKERS") {
                Some(path) => {
                    path.into_string().unwrap().parse::<i32>().unwrap()
                }
                None => 10,
            },
            gateway_timeout: match var_os("GATEWAY_TIMEOUT") {
                Some(path) => {
                    path.into_string().unwrap().parse::<u64>().unwrap()
                }
                None => 5 as u64,
            },
            tls_cert_path: match var_os("TLS_CERT_PATH") {
                Some(path) => Some(path.into_string().unwrap()),
                None => None,
            },
            tls_key_path: match var_os("TLS_KEY_PATH") {
                Some(path) => Some(path.into_string().unwrap()),
                None => None,
            },
            token_secret_key: match var_os("TOKEN_SECRET_KEY") {
                Some(path) => path.into_string().unwrap(),
                None => panic!("TOKEN_SECRET_KEY is not set"),
            },
            token_expiration: match var_os("TOKEN_EXPIRATION") {
                Some(path) => {
                    path.into_string().unwrap().parse::<i64>().unwrap()
                }
                None => 3600,
            },
            token_hmac_secret: match var_os("TOKEN_HMAC_SECRET") {
                Some(path) => path.into_string().unwrap(),
                None => panic!("TOKEN_HMAC_SECRET is not set"),
            },
            token_email_notifier: match var_os("TOKEN_EMAIL_NOTIFIER") {
                Some(email) => {
                    match Email::from_string(email.into_string().unwrap()) {
                        Ok(email) => email,
                        Err(err) => panic!(
                            "TOKEN_EMAIL_NOTIFIER is not a valid email: {err}"
                        ),
                    }
                }
                None => panic!("TOKEN_EMAIL_NOTIFIER is not set"),
            },
        }
    }
} */
