// ? ---------------------------------------------------------------------------
// ? AccountScopedConnectionStringMeta
//
// Data type used to store connection string scoped to an specific account
//
// ? ---------------------------------------------------------------------------

use super::ConnectionStringBean;
use crate::{
    domain::dtos::{
        route_type::PermissionedRoles,
        token::{HmacSha256, ScopedBehavior, ServiceAccountRelatedMeta},
    },
    models::AccountLifeCycle,
};

use chrono::{DateTime, Local};
use hmac::Mac;
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::fmt::Write;
use tracing::error;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountWithPermissionedRolesScope(Vec<ConnectionStringBean>);

impl AccountWithPermissionedRolesScope {
    /// Create a new AccountScope
    ///
    /// Account scope is a list of ConnectionStringBean including the tenant_id,
    /// account_id and the permissioned roles. It also includes a signature
    /// created with the HMAC of the data and the secret from the config.
    ///
    #[tracing::instrument(name = "new", skip(config))]
    pub async fn new(
        tenant_id: Uuid,
        account_id: Uuid,
        permissioned_roles: PermissionedRoles,
        expires_at: DateTime<Local>,
        config: AccountLifeCycle,
    ) -> Result<Self, MappedErrors> {
        let mut self_signed_scope = Self(vec![
            ConnectionStringBean::TID(tenant_id),
            ConnectionStringBean::AID(account_id),
            ConnectionStringBean::PR(permissioned_roles),
            ConnectionStringBean::EDT(expires_at),
        ]);

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

    #[tracing::instrument(name = "get_permissioned_roles", skip(self))]
    fn get_permissioned_roles(&self) -> Option<PermissionedRoles> {
        self.0.iter().find_map(|bean| {
            if let ConnectionStringBean::PR(permissioned_roles) = bean {
                return Some(permissioned_roles.clone());
            }

            None
        })
    }

    #[tracing::instrument(name = "include_tenant", skip(self))]
    fn include_tenant(&self, tenant_id: Uuid) -> bool {
        self.0.iter().any(|bean| {
            if let ConnectionStringBean::TID(id) = bean {
                return *id == tenant_id;
            }

            false
        })
    }
}

impl ScopedBehavior for AccountWithPermissionedRolesScope {
    /// Sign the token with secret and data
    ///
    /// Add or replace a signature to self with the HMAC of the data and the
    /// secret
    ///
    #[tracing::instrument(name = "sign_token", skip(self, config))]
    async fn sign_token(
        &mut self,
        config: AccountLifeCycle,
        extra_data: Option<String>,
    ) -> Result<String, MappedErrors> {
        let secret = config.token_secret.async_get_or_error().await;

        let mut mac = match HmacSha256::new_from_slice(secret?.as_bytes()) {
            Ok(mac) => mac,
            Err(err) => {
                error!("Could not create HMAC: {}", err);
                return dto_err("Unable to sign token").as_error();
            }
        };

        mac.update(self.to_string().as_bytes());
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

impl ToString for AccountWithPermissionedRolesScope {
    fn to_string(&self) -> String {
        self.0
            .iter()
            .fold(String::new(), |acc, bean| {
                format!("{}{};", acc, bean.to_string())
            })
            .trim_end_matches(';')
            .to_string()
    }
}

impl TryFrom<String> for AccountWithPermissionedRolesScope {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let beans = value
            .split(';')
            .map(|bean| ConnectionStringBean::try_from(bean.to_string()))
            .collect::<Result<Vec<ConnectionStringBean>, ()>>()?;

        Ok(Self(beans))
    }
}

pub type AccountScopedConnectionString =
    ServiceAccountRelatedMeta<String, AccountWithPermissionedRolesScope>;

impl AccountScopedConnectionString {
    #[tracing::instrument(name = "get_signature", skip(self))]
    pub fn get_signature(&self) -> Option<String> {
        self.scope.get_signature()
    }

    #[tracing::instrument(name = "get_permissioned_roles", skip(self))]
    pub fn get_permissioned_roles(&self) -> Option<PermissionedRoles> {
        self.scope.get_permissioned_roles()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::{email::Email, guest_role::Permission};

    use myc_config::secret_resolver::SecretResolver;

    /// Test new signed token
    ///
    /// Test the creation of a new signed token with the
    /// AccountScopedConnectionStringMeta struct and test if the signature and
    /// the further password check are correct
    #[tokio::test]
    async fn test_new_signed_token() {
        let config = AccountLifeCycle {
            domain_url: None,
            domain_name: "test".to_string(),
            locale: None,
            token_expiration: 30,
            noreply_name: None,
            noreply_email: SecretResolver::Value("test".to_string()),
            support_name: None,
            support_email: SecretResolver::Value("test".to_string()),
            token_secret: SecretResolver::Value("test".to_string()),
        };

        let account_scope = AccountWithPermissionedRolesScope::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            vec![("role".to_string(), Permission::ReadWrite)],
            Local::now(),
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
            AccountScopedConnectionString::new_signed_token(
                &mut account_scope,
                user_id,
                email,
                config,
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
