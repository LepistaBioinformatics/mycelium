// ? ---------------------------------------------------------------------------
// ? AccountScopedConnectionStringMeta
//
// Data type used to store connection string scoped to an specific account
//
// ? ---------------------------------------------------------------------------

use super::ConnectionStringBean;
use crate::{
    domain::dtos::{
        security_group::PermissionedRole,
        token::{HmacSha256, ScopedBehavior, ServiceAccountRelatedMeta},
    },
    models::AccountLifeCycle,
};

use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local};
use hmac::Mac;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UserAccountScope(Vec<ConnectionStringBean>);

impl UserAccountScope {
    /// Create a new AccountScope
    ///
    /// Account scope is a list of ConnectionStringBean including the tenant_id,
    /// account_id and the permissioned roles. It also includes a signature
    /// created with the HMAC of the data and the secret from the config.
    ///
    #[tracing::instrument(name = "new", skip(config))]
    pub async fn new(
        account_id: Uuid,
        expires_at: DateTime<Local>,
        roles: Option<Vec<PermissionedRole>>,
        tenant_id: Option<Uuid>,
        subscription_account_id: Option<Uuid>,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        let mut beans = vec![
            ConnectionStringBean::AID(account_id),
            ConnectionStringBean::EDT(expires_at),
        ];

        if let Some(roles) = roles {
            beans.push(ConnectionStringBean::RLS(roles));
        }

        if let Some(tenant_id) = tenant_id {
            beans.push(ConnectionStringBean::TID(tenant_id));
        }

        if let Some(subscription_account_id) = subscription_account_id {
            beans.push(ConnectionStringBean::SID(subscription_account_id));
        }

        let mut self_signed_scope = Self(beans);

        self_signed_scope.sign_token(config, None).await?;

        Ok(self_signed_scope)
    }

    /// Get the scope beans
    ///
    /// A simple function to expose the scope beans
    #[tracing::instrument(name = "get_scope_beans", skip(self))]
    pub fn get_scope_beans(&self) -> Vec<ConnectionStringBean> {
        self.0.clone()
    }

    /// Get the scope signature
    ///
    /// Get the signature from the scope if it exists
    #[tracing::instrument(name = "get_signature", skip(self))]
    fn get_signature(&self) -> Option<String> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::SIG(signature) = bean {
                return Some(signature.clone());
            }

            None
        })
    }

    #[tracing::instrument(name = "get_user_account_id", skip(self))]
    pub fn get_user_account_id(&self) -> Option<Uuid> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::AID(id) = bean {
                return Some(*id);
            }

            None
        })
    }

    #[tracing::instrument(name = "get_roles", skip(self))]
    pub fn get_roles(&self) -> Option<Vec<PermissionedRole>> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::RLS(roles) = bean {
                return Some(roles.clone());
            }

            None
        })
    }

    #[tracing::instrument(name = "get_tenant_id", skip(self))]
    pub fn get_tenant_id(&self) -> Option<Uuid> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::TID(id) = bean {
                return Some(*id);
            }

            None
        })
    }

    #[tracing::instrument(name = "get_service_account_id", skip(self))]
    pub fn get_service_account_id(&self) -> Option<Uuid> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::SID(id) = bean {
                return Some(*id);
            }

            None
        })
    }

    /// Verify that the scope's `SIG` bean matches the HMAC of the other
    /// beans under the currently-configured HMAC key.
    ///
    /// Etapa 1 wires the HMAC key resolver (with fallback to
    /// `token_secret`); Etapa 3 will upgrade the key lookup to per-version
    /// (KVR) and add an anti-downgrade guarantee. The method is added now
    /// so the verification path exists as a no-op shim — the request
    /// middleware will start calling it in Etapa 3.
    #[tracing::instrument(name = "verify_signature", skip_all)]
    pub async fn verify_signature(
        &self,
        config: &AccountLifeCycle,
    ) -> Result<(), MappedErrors> {
        let Some(stored_hex) = self.get_signature() else {
            return dto_err("missing signature").as_error();
        };

        let expected = hex::decode(stored_hex.as_bytes()).map_err(|err| {
            dto_err(format!("invalid signature encoding: {err}"))
        })?;

        let payload = serialize_beans_for_hmac(&self.0);
        let key_bytes = config.hmac_signing_key_bytes().await?;

        let mut mac =
            HmacSha256::new_from_slice(&key_bytes).map_err(|err| {
                tracing::error!("Could not create HMAC: {err}");
                dto_err("Unable to verify signature")
            })?;

        mac.update(payload.as_bytes());

        mac.verify_slice(&expected)
            .map_err(|_| dto_err("signature mismatch"))?;

        Ok(())
    }
}

/// Serialise a bean list for HMAC input.
///
/// Filters the `SIG` bean out (so signing and verification agree on the
/// payload) and then applies the same base64-of-joined-pairs encoding as
/// `UserAccountScope::to_string`. Both `sign_token` and `verify_signature`
/// must route through this helper so the two paths stay in lock-step.
fn serialize_beans_for_hmac(beans: &[ConnectionStringBean]) -> String {
    let raw_string = beans
        .iter()
        .filter(|bean| !matches!(bean, ConnectionStringBean::SIG(_)))
        .fold(String::new(), |acc, bean| {
            format!("{}{};", acc, bean.to_string())
        })
        .trim_end_matches(';')
        .to_string();

    general_purpose::STANDARD.encode(raw_string)
}

impl ScopedBehavior for UserAccountScope {
    /// Sign the token with secret and data
    ///
    /// Add or replace a signature to self with the HMAC of the data and the
    /// secret
    ///
    /// The HMAC key is `AccountLifeCycle::hmac_secret`, falling back to
    /// `token_secret` when `hmac_secret` is not configured (Etapa 1+2 of
    /// the HMAC rotation rollout). While the fallback is active, rotating
    /// `token_secret` also invalidates every signature previously produced
    /// here, and there is no re-signing path. See
    /// `AccountLifeCycle::derive_kek_bytes` for the full list of
    /// `token_secret` consumers and rotation caveats.
    #[tracing::instrument(name = "sign_token", skip(self, config))]
    async fn sign_token(
        &mut self,
        config: AccountLifeCycle,
        extra_data: Option<String>,
    ) -> Result<String, MappedErrors> {
        let key_bytes = config.hmac_signing_key_bytes().await?;

        let mut mac = match HmacSha256::new_from_slice(&key_bytes) {
            Ok(mac) => mac,
            Err(err) => {
                tracing::error!("Could not create HMAC: {}", err);
                return dto_err("Unable to sign token").as_error();
            }
        };

        mac.update(serialize_beans_for_hmac(&self.0).as_bytes());
        let result = mac.finalize();

        let hmac_bytes = result.into_bytes();
        let mut hex_string = String::with_capacity(hmac_bytes.len() * 2);
        for byte in hmac_bytes {
            write!(&mut hex_string, "{:02x}", byte).expect("Unable to write");
        }

        self.0 = self
            .0
            .iter()
            .filter(|bean| {
                if let ConnectionStringBean::SIG(_) = bean {
                    return false;
                }

                true
            })
            .cloned()
            .collect();

        self.0.push(ConnectionStringBean::SIG(hex_string.clone()));

        Ok(hex_string)
    }
}

impl ToString for UserAccountScope {
    /// Convert the UserAccountScope to a string
    ///
    /// This function generate a base64 representation of the scope beans
    ///
    fn to_string(&self) -> String {
        //
        // Generate a raw string representation of the scope beans
        //
        let raw_string = self
            .0
            .iter()
            .fold(String::new(), |acc, bean| {
                format!("{}{};", acc, bean.to_string())
            })
            .trim_end_matches(';')
            .to_string();

        //
        // Encode the raw string using base64
        //
        general_purpose::STANDARD.encode(raw_string)
    }
}

impl TryFrom<String> for UserAccountScope {
    type Error = ();

    /// Try to convert a base64 encoded string into a UserAccountScope
    ///
    /// This function decodes the base64 string, converts it into a UTF-8 string
    /// and then parses the string into individual ConnectionStringBeans. If any
    /// step fails, it returns an empty error.
    ///
    fn try_from(value: String) -> Result<Self, Self::Error> {
        //
        // Decode the base64 encoded string resulting into a [u8] vector
        //
        let raw_decoded = match general_purpose::STANDARD.decode(value) {
            Ok(decoded) => decoded,
            Err(err) => {
                tracing::error!("Failed to decode base64 string: {err}");
                return Err(());
            }
        };

        //
        // Convert the raw bytes into a UTF-8 string
        //
        let decoded = String::from_utf8(raw_decoded).map_err(|_| {
            tracing::error!("Failed to convert decoded bytes to UTF-8 string");
        })?;

        //
        // Parse the decoded string into individual beans
        //
        let beans = decoded
            .split(';')
            .map(|bean| ConnectionStringBean::try_from(bean.to_string()))
            .collect::<Result<Vec<ConnectionStringBean>, ()>>()?;

        Ok(Self(beans))
    }
}

pub type UserAccountConnectionString =
    ServiceAccountRelatedMeta<String, UserAccountScope>;

impl UserAccountConnectionString {
    #[tracing::instrument(name = "get_signature", skip(self))]
    pub fn get_signature(&self) -> Option<String> {
        self.scope.get_signature()
    }

    #[tracing::instrument(name = "get_user_account_id", skip(self))]
    pub fn get_user_account_id(&self) -> Option<Uuid> {
        self.scope.get_user_account_id()
    }

    #[tracing::instrument(name = "get_roles", skip(self))]
    pub fn get_roles(&self) -> Option<Vec<PermissionedRole>> {
        self.scope.get_roles()
    }

    #[tracing::instrument(name = "get_tenant_id", skip(self))]
    pub fn get_tenant_id(&self) -> Option<Uuid> {
        self.scope.get_tenant_id()
    }

    #[tracing::instrument(name = "get_service_account_id", skip(self))]
    pub fn get_service_account_id(&self) -> Option<Uuid> {
        self.scope.get_service_account_id()
    }

    //#[tracing::instrument(name = "get_permissioned_roles", skip(self))]
    //pub fn get_permissioned_roles(&self) -> Option<PermissionedRoles> {
    //    self.scope.get_permissioned_roles()
    //}
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::email::Email;

    use myc_config::secret_resolver::SecretResolver;

    // ? -----------------------------------------------------------------
    // ? Helpers
    // ? -----------------------------------------------------------------

    fn base_config() -> AccountLifeCycle {
        AccountLifeCycle {
            domain_url: None,
            domain_name: SecretResolver::Value("test".to_string()),
            locale: None,
            token_expiration: SecretResolver::Value(30),
            noreply_name: None,
            noreply_email: SecretResolver::Value("test".to_string()),
            support_name: None,
            support_email: SecretResolver::Value("test".to_string()),
            token_secret: SecretResolver::Value(
                "fallback-token-secret".to_string(),
            ),
            hmac_secret: None,
        }
    }

    async fn issue_scope(
        config: AccountLifeCycle,
    ) -> Result<UserAccountScope, MappedErrors> {
        UserAccountScope::new(
            Uuid::new_v4(),
            Local::now(),
            None,
            None,
            None,
            config,
        )
        .await
    }

    // ? -----------------------------------------------------------------
    // ? Etapa 1 — sign / verify / fallback coverage
    // ? -----------------------------------------------------------------

    #[tokio::test]
    async fn test_sign_then_verify_with_hmac_secret_present(
    ) -> Result<(), MappedErrors> {
        let mut config = base_config();
        config.hmac_secret =
            Some(SecretResolver::Value("dedicated-hmac-secret".to_string()));

        let scope = issue_scope(config.clone()).await?;
        scope.verify_signature(&config).await?;

        assert!(scope.get_signature().is_some());
        Ok(())
    }

    #[tokio::test]
    async fn test_sign_then_verify_falls_back_to_token_secret(
    ) -> Result<(), MappedErrors> {
        let config = base_config();
        assert!(config.hmac_secret.is_none());

        let scope = issue_scope(config.clone()).await?;
        scope.verify_signature(&config).await?;

        Ok(())
    }

    #[tokio::test]
    async fn test_verify_rejects_tampered_payload() -> Result<(), MappedErrors>
    {
        let config = base_config();
        let scope = issue_scope(config.clone()).await?;

        let mut tampered_beans = scope.get_scope_beans();
        let mut aid_swapped = false;
        for bean in tampered_beans.iter_mut() {
            if let ConnectionStringBean::AID(_) = bean {
                *bean = ConnectionStringBean::AID(Uuid::new_v4());
                aid_swapped = true;
                break;
            }
        }
        assert!(aid_swapped, "expected AID bean in issued scope");

        let tampered_scope = UserAccountScope(tampered_beans);

        let outcome = tampered_scope.verify_signature(&config).await;
        assert!(outcome.is_err(), "tampered scope must fail verification");

        Ok(())
    }

    /// Test new signed token
    ///
    /// Test the creation of a new signed token with the
    /// AccountScopedConnectionStringMeta struct and test if the signature and
    /// the further password check are correct
    #[tokio::test]
    async fn test_new_signed_token() {
        let config = base_config();

        let account_scope = UserAccountScope::new(
            Uuid::new_v4(),
            Local::now(),
            None,
            None,
            None,
            config.to_owned(),
        )
        .await;

        assert!(account_scope.is_ok());

        let mut account_scope = account_scope.unwrap();

        let user_id = Uuid::new_v4();
        let email = Email {
            username: "test".to_string(),
            domain: "test.com".to_string(),
        };

        let account_scoped_connection_string =
            UserAccountConnectionString::new_signed_token(
                &mut account_scope,
                user_id,
                email,
                config,
                None,
            )
            .await;

        assert!(account_scoped_connection_string.is_ok());

        let mut account_scoped_connection_string =
            account_scoped_connection_string.unwrap();

        let signature = account_scoped_connection_string.get_signature();

        assert!(signature.is_some());

        let signature = signature.unwrap();

        let with_encrypted_token =
            account_scoped_connection_string.encrypted_token();

        assert!(with_encrypted_token.is_ok());

        let password_check =
            account_scoped_connection_string.check_token(signature.as_bytes());

        assert!(password_check.is_ok());
    }
}
