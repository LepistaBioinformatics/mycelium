use super::{account::Account, email::Email};
use crate::{
    domain::utils::{decrypt_string_with_dek, encrypt_with_dek},
    models::AccountLifeCycle,
};

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash as Argon2PasswordHash, SaltString,
    },
    Argon2, PasswordHasher, PasswordVerifier,
};
use chrono::{DateTime, Local};
use mycelium_base::{
    dtos::Parent,
    utils::errors::{use_case_err, MappedErrors},
};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PasswordHash {
    #[serde(skip_serializing)]
    pub hash: String,

    #[serde(skip_serializing, skip_deserializing)]
    password: Option<String>,
}

impl PasswordHash {
    pub fn new_from_hash(hash: String) -> Self {
        Self {
            hash,
            password: None,
        }
    }

    pub fn hash_user_password(password: &[u8]) -> Self {
        Self {
            hash: Argon2::default()
                .hash_password(password, &SaltString::generate(&mut OsRng))
                .expect("Unable to hash password.")
                .to_string(),
            password: None,
        }
    }

    pub fn check_password(&self, password: &[u8]) -> Result<(), MappedErrors> {
        let parsed_hash = match Argon2PasswordHash::new(&self.hash) {
            Ok(hash) => hash,
            Err(err) => {
                return use_case_err(format!(
                    "Unable to parse password hash: {err}",
                ))
                .as_error()
            }
        };

        match Argon2::default().verify_password(password, &parsed_hash) {
            Ok(_) => Ok(()),
            Err(err) => use_case_err(format!("Unable to verify secret: {err}"))
                .with_exp_true()
                .as_error(),
        }
    }

    pub fn get_raw_password(&self) -> Option<String> {
        self.password.to_owned()
    }

    pub fn with_raw_password(&mut self, password: String) -> Self {
        self.password = Some(password);
        self.to_owned()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    External(String),
    Internal(PasswordHash),
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Totp {
    Unknown,

    Disabled,

    /// The TOTP when enabled
    ///
    /// The TOTP is enabled when the user has verified the TOTP and the auth
    /// url is set. The secret is not serialized to avoid that the secret is
    /// exposed to the outside.
    ///
    #[serde(rename_all = "camelCase")]
    Enabled {
        verified: bool,
        issuer: String,
        secret: Option<String>,
    },
}

impl Totp {
    #[tracing::instrument(name = "build_auth_url", skip_all)]
    pub(crate) async fn build_auth_url(
        &self,
        email: Email,
        dek: &[u8; 32],
        config: &AccountLifeCycle,
        aad: &[u8],
    ) -> Result<String, MappedErrors> {
        let mut self_copy = self.clone();
        self_copy = self_copy.decrypt_me(dek, config, aad).await?;

        let (secret, issuer) = match self_copy {
            Self::Enabled { issuer, secret, .. } => match secret {
                Some(secret) => (secret, issuer.to_owned()),
                None => {
                    return use_case_err("Totp is enabled but secret is None.")
                        .as_error()
                }
            },
            _ => {
                return use_case_err(
                    "Totp is disabled and should not be enabled.",
                )
                .as_error()
            }
        };

        Ok(format!(
            "otpauth://totp/{issuer}:{email}?secret={secret}&issuer={issuer}",
            issuer = issuer,
            email = email.email(),
            secret = secret
        ))
    }

    /// Encrypt the TOTP secret with the tenant DEK (v2 format).
    #[tracing::instrument(name = "encrypt_secret", skip_all)]
    pub(crate) fn encrypt_me(
        &self,
        dek: &[u8; 32],
        aad: &[u8],
    ) -> Result<Self, MappedErrors> {
        let plaintext_secret = match self {
            Self::Enabled {
                secret: Some(secret),
                ..
            } => secret.as_str(),
            _ => {
                return use_case_err("Totp is not enabled or secret is missing")
                    .as_error()
            }
        };

        let encrypted_string = encrypt_with_dek(plaintext_secret, dek, aad)?;

        Ok(Self::Enabled {
            verified: matches!(
                self,
                Self::Enabled { verified, .. } if *verified
            ),
            issuer: match self {
                Self::Enabled { issuer, .. } => issuer.clone(),
                _ => return use_case_err("Expected enabled Totp").as_error(),
            },
            secret: Some(encrypted_string),
        })
    }

    /// Decrypt the TOTP secret.
    ///
    /// Detects v1 (no prefix) or v2 (`v2:` prefix) automatically. v2 uses the
    /// supplied `dek`; v1 falls back to the legacy KEK path via `config`.
    ///
    /// The v1 fallback derives the KEK directly from
    /// `AccountLifeCycle::token_secret`. Any TOTP secret still in v1 format
    /// becomes unreadable if `token_secret` is rotated before `migrate-dek`
    /// completes. See `AccountLifeCycle::derive_kek_bytes` for the full list
    /// of `token_secret` consumers and rotation caveats.
    #[tracing::instrument(name = "decrypt_secret", skip_all)]
    pub(crate) async fn decrypt_me(
        &self,
        dek: &[u8; 32],
        config: &AccountLifeCycle,
        aad: &[u8],
    ) -> Result<Self, MappedErrors> {
        let secret = match self {
            Self::Enabled {
                secret: Some(secret),
                ..
            } => secret,
            _ => {
                return use_case_err("Totp is not enabled or secret is missing")
                    .as_error()
            }
        };

        let decrypted_secret =
            decrypt_string_with_dek(secret, config, dek, aad).await?;

        Ok(Self::Enabled {
            verified: matches!(
                self,
                Self::Enabled { verified, .. } if *verified
            ),
            issuer: match self {
                Self::Enabled { issuer, .. } => issuer.clone(),
                _ => return use_case_err("Expected enabled Totp").as_error(),
            },
            secret: Some(decrypted_secret),
        })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MultiFactorAuthentication {
    /// The TOTP
    ///
    /// The TOTP is disabled by default.
    ///
    pub totp: Totp,
}

impl MultiFactorAuthentication {
    pub fn redact_secrets(&mut self) -> Self {
        if let Totp::Enabled {
            verified, issuer, ..
        } = &self.totp
        {
            self.totp = Totp::Enabled {
                verified: *verified,
                issuer: issuer.to_owned(),
                secret: Some("REDACTED".to_string()),
            }
        }

        self.to_owned()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct User {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<Uuid>,

    pub username: String,

    pub email: Email,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_name: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_name: Option<String>,

    pub is_active: bool,

    pub created: DateTime<Local>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<DateTime<Local>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub account: Option<Parent<Account, Uuid>>,

    /// If the user is the principal user of the account.
    ///
    /// The principal user contains information of the first email that created
    /// the account. This information is used to send emails to the principal
    /// user.
    ///
    /// Principal users should not be deleted or deactivated if the account has
    /// other users connected.
    ///
    is_principal: bool,

    /// The user provider.
    ///
    /// Provider is a optional field but it should be None only during the
    /// collection of the user data from database. Such None initialization
    /// prevents that password hashes and salts should be exposed to the
    /// outside.
    ///
    /// ! Thus, be careful on change this field.
    ///
    provider: Option<Provider>,

    /// The user TOTP
    ///
    /// When enabled the user has verified the TOTP and the auth url is set.
    ///
    mfa: MultiFactorAuthentication,
}

impl Serialize for User {
    /// This method is required to avoid that the password hash and salt are
    /// exposed to the outside.
    ///
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: ::serde::ser::Serializer,
    {
        let mut user = self.clone();
        user.provider = match self.provider.to_owned() {
            Some(Provider::Internal(_)) => Some(Provider::Internal(
                PasswordHash::new_from_hash("".to_string()),
            )),
            Some(Provider::External(external)) => {
                Some(Provider::External(external))
            }
            None => None,
        };

        user.mfa = user.mfa.redact_secrets();

        let mut state = serializer.serialize_struct("User", 12)?;

        if user.id.is_some() {
            state.serialize_field("id", &user.id)?;
        }

        if user.first_name.is_some() {
            state.serialize_field("firstName", &user.first_name)?;
        }

        if user.last_name.is_some() {
            state.serialize_field("lastName", &user.last_name)?;
        }

        state.serialize_field("username", &user.username)?;

        state.serialize_field("email", &user.email)?;

        state.serialize_field("isActive", &user.is_active)?;

        state.serialize_field("isPrincipal", &user.is_principal)?;

        state.serialize_field("created", &user.created)?;

        if user.updated.is_some() {
            state.serialize_field("updated", &user.updated)?;
        }

        if user.account.is_some() {
            state.serialize_field("account", &user.account)?;
        }

        if user.provider.is_some() {
            state.serialize_field("provider", &user.provider)?;
        }

        state.serialize_field("mfa", &user.mfa)?;

        state.end()
    }
}

impl User {
    // ? -----------------------------------------------------------------------
    // ? Constructors
    // ? -----------------------------------------------------------------------

    fn new_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
        is_principal: bool,
    ) -> Result<Self, MappedErrors> {
        Ok(Self {
            id: None,
            username: match username {
                Some(username) => username,
                None => email.to_owned().username,
            },
            email,
            first_name,
            last_name,
            provider: Some(provider),
            is_active: true,
            is_principal,
            created: Local::now(),
            updated: None,
            account: None,
            mfa: MultiFactorAuthentication {
                totp: Totp::Disabled,
            },
        })
    }

    pub fn new_principal_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<Self, MappedErrors> {
        Self::new_with_provider(
            username, email, provider, first_name, last_name, true,
        )
    }

    pub fn new_secondary_with_provider(
        username: Option<String>,
        email: Email,
        provider: Provider,
        first_name: Option<String>,
        last_name: Option<String>,
    ) -> Result<Self, MappedErrors> {
        Self::new_with_provider(
            username, email, provider, first_name, last_name, false,
        )
    }

    pub fn new(
        id: Option<Uuid>,
        username: String,
        email: Email,
        first_name: Option<String>,
        last_name: Option<String>,
        is_active: bool,
        created: DateTime<Local>,
        updated: Option<DateTime<Local>>,
        account: Option<Parent<Account, Uuid>>,
        provider: Option<Provider>,
    ) -> Self {
        Self {
            id,
            username,
            email,
            first_name,
            last_name,
            is_active,
            created,
            updated,
            account,
            provider,
            is_principal: false,
            mfa: MultiFactorAuthentication {
                totp: Totp::Disabled,
            },
        }
    }

    pub fn new_public_redacted(
        id: Uuid,
        email: Email,
        username: String,
        created: DateTime<Local>,
        is_active: bool,
        is_principal: bool,
    ) -> Self {
        Self {
            id: Some(id),
            username,
            email,
            first_name: None,
            last_name: None,
            is_active,
            created,
            updated: None,
            account: None,
            is_principal,
            provider: None,
            mfa: MultiFactorAuthentication {
                totp: Totp::Unknown,
            },
        }
    }

    // ? -----------------------------------------------------------------------
    // ? Instance methods
    // ? -----------------------------------------------------------------------

    pub fn with_principal(&mut self, is_principal: bool) -> Self {
        self.is_principal = is_principal.to_owned();
        self.to_owned()
    }

    pub fn with_mfa(&mut self, mfa: MultiFactorAuthentication) -> Self {
        self.mfa = mfa;
        self.to_owned()
    }

    pub fn is_principal(&self) -> bool {
        self.is_principal
    }

    pub fn provider(&self) -> Option<Provider> {
        self.provider.to_owned()
    }

    pub fn mfa(&self) -> MultiFactorAuthentication {
        self.mfa.to_owned()
    }

    /// Try to get the provider kind or return an error
    ///
    /// This method should be used to check if the user is registered in
    /// Mycelium with an internal provider or not.
    pub fn with_internal_provider(&self) -> Result<bool, MappedErrors> {
        match self.provider {
            Some(Provider::Internal(_)) => Ok(true),
            Some(Provider::External(_)) => Ok(false),
            None => use_case_err(
                "User is probably registered but mycelium is unable to
check if user is internal or not. The user provider is None.",
            )
            .as_error(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{domain::utils::generate_dek, models::AccountLifeCycle};
    use myc_config::secret_resolver::SecretResolver;

    fn make_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_name: SecretResolver::Value("test".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(30),
            noreply_name: None,
            noreply_email: SecretResolver::Value("test".to_string()),
            support_name: None,
            support_email: SecretResolver::Value("test".to_string()),
            token_secret: SecretResolver::Value(
                "ab4c0550-310b-4218-9edf-58edc87979b9".to_string(),
            ),
            hmac_secret: None,
        }
    }

    #[tokio::test]
    async fn test_encrypt_and_decrypt_totp_secret_v2() {
        let secret = "secret";
        let issuer = "issuer";
        let totp = Totp::Enabled {
            verified: true,
            issuer: issuer.to_string(),
            secret: Some(secret.to_string()),
        };

        let config = make_config();
        let dek = generate_dek().unwrap();
        let aad = b"tenant-test-aad" as &[u8];

        let encrypted = totp.encrypt_me(&dek, aad);
        assert!(encrypted.is_ok());

        let enc = encrypted.unwrap();
        let secret_field = match &enc {
            Totp::Enabled { secret, .. } => secret.as_ref().unwrap(),
            _ => panic!("Expected Enabled variant"),
        };
        assert!(secret_field.starts_with("v2:"));

        let decrypted = enc.decrypt_me(&dek, &config, aad).await;
        assert!(decrypted.is_ok());
        assert_eq!(totp, decrypted.unwrap());
    }
}
