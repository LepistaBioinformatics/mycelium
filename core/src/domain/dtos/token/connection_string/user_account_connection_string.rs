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
}

impl ScopedBehavior for UserAccountScope {
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
                tracing::error!("Could not create HMAC: {}", err);
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

    /// Test new signed token
    ///
    /// Test the creation of a new signed token with the
    /// AccountScopedConnectionStringMeta struct and test if the signature and
    /// the further password check are correct
    #[tokio::test]
    async fn test_new_signed_token() {
        let config = AccountLifeCycle {
            domain_url: None,
            domain_name: SecretResolver::Value("test".to_string()),
            locale: None,
            token_expiration: SecretResolver::Value(30),
            noreply_name: None,
            noreply_email: SecretResolver::Value("test".to_string()),
            support_name: None,
            support_email: SecretResolver::Value("test".to_string()),
            token_secret: SecretResolver::Value("test".to_string()),
        };

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
