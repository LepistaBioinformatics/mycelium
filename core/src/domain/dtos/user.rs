use super::{account::Account, email::Email};
use crate::{domain::utils::derive_key_from_uuid, models::AccountLifeCycle};

use argon2::{
    password_hash::{
        rand_core::OsRng, PasswordHash as Argon2PasswordHash, SaltString,
    },
    Argon2, PasswordHasher, PasswordVerifier,
};
use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local};
use futures::executor::block_on;
use mycelium_base::{
    dtos::Parent,
    utils::errors::{dto_err, use_case_err, MappedErrors},
};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};
use serde::{ser::SerializeStruct, Deserialize, Serialize};
use tracing::error;
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
            Err(err) => {
                use_case_err(format!("Unable to verify password: {err}"))
                    .with_exp_true()
                    .as_error()
            }
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
    pub(crate) fn build_auth_url(
        &self,
        email: Email,
        config: AccountLifeCycle,
    ) -> Result<String, MappedErrors> {
        let mut self_copy = self.clone();
        self_copy = self_copy.decrypt_me(config)?;

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
            email = email.get_email(),
            secret = secret
        ))
    }

    #[tracing::instrument(name = "encrypt_secret", skip_all)]
    pub(crate) fn encrypt_me(
        &self,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        //
        // Create a key from the account's secret
        //
        let encryption_key = block_on(config.token_secret.async_get_or_error());
        let encryption_key_uuid = match Uuid::parse_str(&encryption_key?) {
            Ok(uuid) => uuid,
            Err(err) => {
                error!("Failed to parse encryption key: {:?}", err);
                return dto_err("Failed to parse encryption key").as_error();
            }
        };

        let key_bytes = derive_key_from_uuid(&encryption_key_uuid);

        let unbound_key = match UnboundKey::new(&AES_256_GCM, &key_bytes) {
            Ok(key) => key,
            Err(err) => {
                error!("Failed to create unbound key: {:?}", err);
                return dto_err("Failed to create unbound key").as_error();
            }
        };

        let key = LessSafeKey::new(unbound_key);

        //
        // Generate a nonce
        //
        let rand = SystemRandom::new();
        let mut nonce_bytes = [0u8; 12];
        match rand.fill(&mut nonce_bytes) {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to generate nonce: {:?}", err);
                return dto_err("Failed to generate nonce").as_error();
            }
        };

        let nonce = Nonce::assume_unique_for_key(nonce_bytes);

        //
        // Prepare secret data to encrypt
        //
        let mut in_out = match self {
            Self::Enabled {
                secret: Some(secret),
                ..
            } => secret.as_bytes().to_vec(),
            _ => {
                return use_case_err("Totp is not enabled or secret is missing")
                    .as_error()
            }
        };

        //
        // Encrypt in-place and append the authentication tag
        //
        match key.seal_in_place_append_tag(nonce, Aad::empty(), &mut in_out) {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to encrypt data: {:?}", err);
                return dto_err("Failed to encrypt data").as_error();
            }
        };

        //
        // Combine nonce and ciphertext for storage
        //
        let mut encrypted_data = nonce_bytes.to_vec();

        encrypted_data.extend_from_slice(&in_out);

        let encrypted_string = general_purpose::STANDARD.encode(encrypted_data);

        //
        // Return encrypted TOTP instance
        //
        let encrypted_totp = Self::Enabled {
            verified: matches!(self, Self::Enabled { verified, .. } if *verified),
            issuer: match self {
                Self::Enabled { issuer, .. } => issuer.clone(),
                _ => return use_case_err("Expected enabled Totp").as_error(),
            },
            secret: Some(encrypted_string),
        };

        Ok(encrypted_totp)
    }

    #[tracing::instrument(name = "decrypt_secret", skip_all)]
    pub(crate) fn decrypt_me(
        &self,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        //
        // Create a key from the account's secret
        //
        let encryption_key = block_on(config.token_secret.async_get_or_error());
        let encryption_key_uuid = match Uuid::parse_str(&encryption_key?) {
            Ok(uuid) => uuid,
            Err(err) => {
                error!("Failed to parse encryption key: {:?}", err);
                return dto_err("Failed to parse encryption key").as_error();
            }
        };

        let key_bytes = derive_key_from_uuid(&encryption_key_uuid);

        let unbound_key = match UnboundKey::new(&AES_256_GCM, &key_bytes) {
            Ok(key) => key,
            Err(err) => {
                error!("Failed to create unbound key: {:?}", err);
                return dto_err("Failed to create unbound key").as_error();
            }
        };

        let key = LessSafeKey::new(unbound_key);

        //
        // Extract and decode the encrypted secret
        //
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

        let encrypted = match general_purpose::STANDARD.decode(secret) {
            Ok(encrypted) => encrypted,
            Err(err) => {
                error!("Failed to decode encrypted data: {:?}", err);
                return dto_err("Failed to decode encrypted data").as_error();
            }
        };

        //
        // Verify that the encrypted data is long enough to contain the nonce
        //
        if encrypted.len() < 12 {
            return dto_err("Encrypted data is too short").as_error();
        }

        //
        // Split encrypted data into nonce and ciphertext
        //
        let (nonce_bytes, ciphertext) = encrypted.split_at(12);

        let nonce = match Nonce::try_assume_unique_for_key(nonce_bytes) {
            Ok(nonce) => nonce,
            Err(_) => {
                return dto_err("Invalid nonce").as_error();
            }
        };

        let mut in_out = ciphertext.to_vec();

        match key.open_in_place(nonce, Aad::empty(), &mut in_out) {
            Ok(_) => {}
            Err(err) => {
                error!("Failed to decrypt data: {:?}", err);
                return dto_err("Failed to decrypt data").as_error();
            }
        };

        let in_out_slice = if in_out.len() > 16 {
            in_out.truncate(in_out.len() - 16);
            in_out
        } else {
            in_out
        };

        //
        // Convert decrypted data from UTF-8 to String
        //
        let decrypted_secret = match String::from_utf8(in_out_slice) {
            Ok(secret) => secret,
            Err(err) => {
                return dto_err(format!(
                    "Failed to convert decrypted data to string: {err}"
                ))
                .as_error();
            }
        };

        let decrypted_totp = Self::Enabled {
            verified: matches!(self, Self::Enabled { verified, .. } if *verified),
            issuer: match self {
                Self::Enabled { issuer, .. } => issuer.clone(),
                _ => return use_case_err("Expected enabled Totp").as_error(),
            },
            secret: Some(decrypted_secret),
        };

        Ok(decrypted_totp)
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
        match &self.totp {
            Totp::Enabled {
                verified,
                issuer,
                secret: _,
            } => {
                self.totp = Totp::Enabled {
                    verified: *verified,
                    issuer: issuer.to_owned(),
                    secret: Some("REDACTED".to_string()),
                }
            }
            _ => {}
        }

        self.to_owned()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, ToSchema, ToResponse)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: Option<Uuid>,

    pub username: String,
    pub email: Email,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub is_active: bool,
    pub created: DateTime<Local>,
    pub updated: Option<DateTime<Local>>,
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
        state.serialize_field("id", &user.id)?;
        state.serialize_field("username", &user.username)?;
        state.serialize_field("email", &user.email)?;
        state.serialize_field("firstName", &user.first_name)?;
        state.serialize_field("lastName", &user.last_name)?;
        state.serialize_field("isActive", &user.is_active)?;
        state.serialize_field("isPrincipal", &user.is_principal)?;
        state.serialize_field("created", &user.created)?;
        state.serialize_field("updated", &user.updated)?;
        state.serialize_field("account", &user.account)?;
        state.serialize_field("provider", &user.provider)?;
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

    /// Check if the user has a provider or not.
    ///
    /// This method should be used to check if the user is registered in
    /// Mycelium with an internal provider or not.
    pub fn has_provider_or_error(&self) -> Result<bool, MappedErrors> {
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
    use crate::models::AccountLifeCycle;

    use myc_config::secret_resolver::SecretResolver;

    #[test]
    fn test_encrypt_and_decrypt_totp_secret() {
        let secret = "secret";
        let issuer = "issuer";
        let totp = Totp::Enabled {
            verified: true,
            issuer: issuer.to_string(),
            secret: Some(secret.to_string()),
        };

        let config = AccountLifeCycle {
            domain_name: SecretResolver::Value("test".to_string()),
            domain_url: None,
            locale: None,
            token_expiration: SecretResolver::Value(30),
            noreply_name: None,
            noreply_email: SecretResolver::Value("test".to_string()),
            support_name: None,
            support_email: SecretResolver::Value("test".to_string()),
            token_secret: SecretResolver::Value("test".to_string()),
        };

        let encrypted = totp.encrypt_me(config.to_owned());

        assert!(encrypted.is_ok());

        let decrypted = encrypted.unwrap().decrypt_me(config);

        assert!(decrypted.is_ok());

        assert_eq!(totp, decrypted.unwrap());
    }
}
