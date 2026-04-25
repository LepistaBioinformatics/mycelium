// ? ---------------------------------------------------------------------------
// ? AccountScopedConnectionStringMeta
//
// Data type used to store connection string scoped to an specific account
//
// ? ---------------------------------------------------------------------------

use super::ConnectionStringBean;
use crate::{
    domain::dtos::{
        native_error_codes::NativeErrorCodes,
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

        beans.push(ConnectionStringBean::KVR(config.hmac_primary_version));

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

    /// Return the HMAC key version (KVR bean) carried by the scope, if
    /// any.
    #[tracing::instrument(name = "get_kvr", skip(self))]
    fn get_kvr(&self) -> Option<u32> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::KVR(version) = bean {
                return Some(*version);
            }

            None
        })
    }

    /// Verify that the scope's `SIG` bean matches the HMAC of the other
    /// beans under the HMAC key identified by the `KVR` bean.
    ///
    /// Failure modes map to native error codes so callers (middleware,
    /// audit logs) can discriminate reasons:
    ///
    /// - `MYC00030` — the token is missing the `KVR` bean.
    /// - `MYC00031` — the version referenced by `KVR` is not in the
    ///   configured key set (retired or never provisioned).
    /// - `MYC00032` — HMAC recomputation does not match the stored `SIG`.
    ///
    /// The KVR value is part of the HMAC input (via
    /// `serialize_beans_for_hmac`), so tampering with it yields
    /// `MYC00032` rather than a false success.
    #[tracing::instrument(name = "verify_signature", skip_all)]
    pub async fn verify_signature(
        &self,
        config: &AccountLifeCycle,
    ) -> Result<(), MappedErrors> {
        let Some(stored_hex) = self.get_signature() else {
            return dto_err("connection_string_missing_signature")
                .with_code(NativeErrorCodes::MYC00030)
                .with_exp_true()
                .as_error();
        };

        let Some(version) = self.get_kvr() else {
            return dto_err("connection_string_missing_key_version")
                .with_code(NativeErrorCodes::MYC00030)
                .with_exp_true()
                .as_error();
        };

        let expected = hex::decode(stored_hex.as_bytes()).map_err(|err| {
            dto_err(format!("invalid_signature_encoding: {err}"))
                .with_code(NativeErrorCodes::MYC00032)
                .with_exp_true()
        })?;

        let key_bytes = config
            .hmac_signing_key_for_version(version)
            .await
            .map_err(|_| {
                dto_err(format!("hmac_key_version_not_configured: {version}",))
                    .with_code(NativeErrorCodes::MYC00031)
                    .with_exp_true()
            })?;

        let payload = serialize_beans_for_hmac(&self.0);

        let mut mac =
            HmacSha256::new_from_slice(&key_bytes).map_err(|err| {
                tracing::error!("Could not create HMAC: {err}");
                dto_err("unable_to_verify_signature")
                    .with_code(NativeErrorCodes::MYC00032)
            })?;

        mac.update(payload.as_bytes());

        mac.verify_slice(&expected).map_err(|_| {
            dto_err("connection_string_signature_mismatch")
                .with_code(NativeErrorCodes::MYC00032)
                .with_exp_true()
        })?;

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
    /// secret.
    ///
    /// The HMAC key is read from `AccountLifeCycle::hmac_primary_signing_key`,
    /// which returns the `(version, key_bytes)` pair tied to the current
    /// `hmac_primary_version`. `UserAccountScope::new` pushes a matching
    /// `KVR` bean **before** this method runs so the version is included in
    /// the HMAC input (anti-downgrade guarantee — see
    /// `docs/book/src/22-hmac-key-rotation.md`).
    #[tracing::instrument(name = "sign_token", skip(self, config))]
    async fn sign_token(
        &mut self,
        config: AccountLifeCycle,
        extra_data: Option<String>,
    ) -> Result<String, MappedErrors> {
        let (_version, key_bytes) = config.hmac_primary_signing_key().await?;

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
    use crate::{
        domain::dtos::email::Email,
        models::{HmacSecretEntry, HmacSecretSet},
    };

    use myc_config::secret_resolver::SecretResolver;

    // ? -----------------------------------------------------------------
    // ? Helpers
    // ? -----------------------------------------------------------------

    fn make_entry(version: u32, value: &str) -> HmacSecretEntry {
        HmacSecretEntry {
            version,
            secret: SecretResolver::Value(value.to_string()),
        }
    }

    fn base_config_with_versions(
        primary: u32,
        entries: Vec<HmacSecretEntry>,
    ) -> AccountLifeCycle {
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
                "ab4c0550-310b-4218-9edf-58edc87979b9".to_string(),
            ),
            hmac_primary_version: primary,
            hmac_secrets: HmacSecretSet::new(entries),
        }
    }

    fn base_config() -> AccountLifeCycle {
        base_config_with_versions(1, vec![make_entry(1, "k-v1")])
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
    // ? Etapa 3 — KVR contract coverage
    // ? -----------------------------------------------------------------

    #[tokio::test]
    async fn signs_with_primary_then_verifies() -> Result<(), MappedErrors> {
        let config = base_config_with_versions(
            2,
            vec![make_entry(1, "k-v1"), make_entry(2, "k-v2")],
        );

        let scope = issue_scope(config.clone()).await?;
        scope.verify_signature(&config).await?;

        assert_eq!(
            scope.get_kvr(),
            Some(2),
            "issued scope must carry KVR=primary",
        );
        assert!(scope.get_signature().is_some());
        Ok(())
    }

    #[tokio::test]
    async fn verifies_non_primary_known_version() -> Result<(), MappedErrors> {
        let issuing_config =
            base_config_with_versions(1, vec![make_entry(1, "k-v1")]);

        let scope = issue_scope(issuing_config).await?;
        assert_eq!(scope.get_kvr(), Some(1));

        let rotated_config = base_config_with_versions(
            2,
            vec![make_entry(1, "k-v1"), make_entry(2, "k-v2")],
        );

        scope.verify_signature(&rotated_config).await?;
        Ok(())
    }

    #[tokio::test]
    async fn rejects_unknown_version() -> Result<(), MappedErrors> {
        let issuing_config = base_config_with_versions(
            2,
            vec![make_entry(1, "k-v1"), make_entry(2, "k-v2")],
        );
        let scope = issue_scope(issuing_config).await?;
        assert_eq!(scope.get_kvr(), Some(2));

        let retired_config =
            base_config_with_versions(1, vec![make_entry(1, "k-v1")]);

        let outcome = scope.verify_signature(&retired_config).await;
        let err = outcome.expect_err("retired key must fail verification");
        assert_eq!(err.code().to_string(), "MYC00031");
        Ok(())
    }

    #[tokio::test]
    async fn rejects_missing_kvr() -> Result<(), MappedErrors> {
        let config = base_config();
        let scope = issue_scope(config.clone()).await?;

        let stripped: Vec<ConnectionStringBean> = scope
            .get_scope_beans()
            .into_iter()
            .filter(|bean| !matches!(bean, ConnectionStringBean::KVR(_)))
            .collect();

        let scope_without_kvr = UserAccountScope(stripped);
        let outcome = scope_without_kvr.verify_signature(&config).await;

        let err = outcome.expect_err("missing KVR must fail verification");
        assert_eq!(err.code().to_string(), "MYC00030");
        Ok(())
    }

    #[tokio::test]
    async fn rejects_tampered_kvr_anti_downgrade() -> Result<(), MappedErrors> {
        let config = base_config_with_versions(
            2,
            vec![make_entry(1, "k-v1"), make_entry(2, "k-v2")],
        );
        let scope = issue_scope(config.clone()).await?;

        let tampered: Vec<ConnectionStringBean> = scope
            .get_scope_beans()
            .into_iter()
            .map(|bean| match bean {
                ConnectionStringBean::KVR(_) => ConnectionStringBean::KVR(1),
                other => other,
            })
            .collect();

        let tampered_scope = UserAccountScope(tampered);
        let outcome = tampered_scope.verify_signature(&config).await;

        let err =
            outcome.expect_err("downgrade attempt must fail verification");
        assert_eq!(err.code().to_string(), "MYC00032");
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
