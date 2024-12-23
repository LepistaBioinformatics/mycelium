use crate::{domain::utils::derive_key_from_uuid, models::AccountLifeCycle};

use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use ring::{
    aead::{Aad, LessSafeKey, Nonce, UnboundKey, AES_256_GCM},
    rand::{SecureRandom, SystemRandom},
};
use serde::{Deserialize, Serialize};
use tracing::error;
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum HttpSecret {
    /// Authentication header
    ///
    /// The secret is passed as an authentication header.
    ///
    #[serde(rename_all = "camelCase")]
    AuthorizationHeader {
        /// The header name
        ///
        /// The name of the header. For example, if the name is `Authorization`,
        /// the header will be `Authorization Bear: <token value>`. The default
        /// value is `Authorization`.
        ///
        #[serde(default = "default_authorization_key")]
        name: Option<String>,

        /// The header prefix
        ///
        /// If present the prefix is added to the header. For example, if the
        /// prefix is `Bearer`, the header will be `Authorization Bearer: <token
        /// value>`.
        ///
        prefix: Option<String>,

        /// The header token
        ///
        /// The token is the value of the header. For example, if the token is
        /// `1234`, the header will be `Authorization Bearer: 123
        ///
        token: String,
    },

    #[serde(rename_all = "camelCase")]
    QueryParameter {
        /// The query parameter name
        ///
        /// The name of the query parameter. For example, if the name is `token`,
        /// the query parameter will be `?token=<token value>`.
        ///
        name: String,

        /// The query parameter value
        ///
        /// The value of the query parameter. For example, if the value is `1234`,
        /// the query parameter will be `?token=1234`.
        ///
        token: String,
    },
    //
    // TODO: Implement client certificate authentication
    //
    //#[serde(rename_all = "camelCase")]
    //ClientCertificate {
    //    /// The certificate
    //    ///
    //    /// The certificate is the client certificate in PEM format.
    //    ///
    //    cert_pem: String,
    //
    //    /// The private key
    //    ///
    //    /// The private key is the client private key in PEM format.
    //    ///
    //    key_pem: String,
    //
    //    /// The certificate version
    //    ///
    //    /// The certificate version is the version of the certificate.
    //    ///
    //    cert_version: Option<String>,
    //},
    //
}

fn default_authorization_key() -> Option<String> {
    Some("Authorization".to_string())
}

impl HttpSecret {
    #[tracing::instrument(name = "encrypt_me", skip_all)]
    pub(crate) fn encrypt_me(
        &self,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        //
        // Create a key from the account's secret
        //
        let encryption_key = config.get_secret()?;
        let key_bytes = derive_key_from_uuid(&encryption_key);

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
        let mut in_out = (match self {
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
        })
        .as_bytes()
        .to_vec();

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
        let self_encrypted = match self {
            Self::AuthorizationHeader { name, prefix, .. } => {
                Self::AuthorizationHeader {
                    token: encrypted_string.to_owned(),
                    name: name.to_owned(),
                    prefix: prefix.to_owned(),
                }
            }
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token: encrypted_string.to_owned(),
                name: name.to_owned(),
            },
        };

        Ok(self_encrypted)
    }

    #[tracing::instrument(name = "decrypt_me", skip_all)]
    pub(crate) fn decrypt_me(
        &self,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        //
        // Create a key from the account's secret
        //
        let encryption_key = config.get_secret()?;
        let key_bytes = derive_key_from_uuid(&encryption_key);

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
            Self::AuthorizationHeader { token, .. } => token,
            Self::QueryParameter { token, .. } => token,
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

        let self_decrypted = match self {
            Self::AuthorizationHeader { name, prefix, .. } => {
                Self::AuthorizationHeader {
                    token: decrypted_secret.to_owned(),
                    name: name.to_owned(),
                    prefix: prefix.to_owned(),
                }
            }
            Self::QueryParameter { name, .. } => Self::QueryParameter {
                token: decrypted_secret,
                name: name.to_owned(),
            },
        };

        Ok(self_decrypted)
    }

    #[tracing::instrument(name = "redact_token", skip_all)]
    pub(crate) fn redact_token(&mut self) {
        let redacted_word = "REDACTED".to_string();

        match self {
            Self::AuthorizationHeader { token, .. } => {
                *token = redacted_word;
            }
            Self::QueryParameter { token, .. } => {
                *token = redacted_word;
            }
        }
    }
}
