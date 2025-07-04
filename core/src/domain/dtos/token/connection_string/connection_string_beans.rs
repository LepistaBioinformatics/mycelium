use std::str::FromStr;

use crate::domain::dtos::{
    guest_role::Permission, security_group::PermissionedRoles,
};

use chrono::{DateTime, Local, Timelike};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ConnectionStringBean {
    /// The signature
    SIG(String),

    /// The expiration date time
    EDT(DateTime<Local>),

    /// The tenant ID
    TID(Uuid),

    /// The account ID
    AID(Uuid),

    /// A service account ID
    SID(Uuid),

    /// A list of roles slugs
    RLS(Vec<String>),

    /// The permission
    PM(Permission),

    /// The permissioned roles
    PR(PermissionedRoles),

    /// The endpoint URL
    URL(String),
}

impl ToString for ConnectionStringBean {
    fn to_string(&self) -> String {
        match self {
            ConnectionStringBean::SIG(signature) => {
                format!("sig={}", signature)
            }
            ConnectionStringBean::EDT(expiration_date) => {
                format!(
                    "edt={}",
                    expiration_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
                )
            }
            ConnectionStringBean::TID(tenant_id) => {
                format!("tid={}", tenant_id.to_string())
            }
            ConnectionStringBean::AID(account_id) => {
                format!("aid={}", account_id.to_string())
            }
            ConnectionStringBean::SID(subscription_account_id) => {
                format!("sid={}", subscription_account_id.to_string())
            }
            ConnectionStringBean::RLS(roles) => {
                let roles = roles
                    .iter()
                    .map(|role| role.to_string())
                    .collect::<Vec<String>>()
                    .join(",");

                format!("rls={}", roles)
            }
            ConnectionStringBean::PM(permission) => {
                format!("pm={}", permission.to_string())
            }
            ConnectionStringBean::PR(permissioned_roles) => {
                let roles = permissioned_roles
                    .iter()
                    .fold(String::new(), |acc, (role, permission)| {
                        format!("{}{}:{},", acc, role, permission.to_i32())
                    })
                    .trim_end_matches(',')
                    .to_string();

                format!("pr={}", roles)
            }
            ConnectionStringBean::URL(endpoint) => {
                format!("url={}", endpoint)
            }
        }
    }
}

impl TryFrom<String> for ConnectionStringBean {
    type Error = ();

    fn try_from(value: String) -> Result<Self, Self::Error> {
        let parts = value.split('=').collect::<Vec<&str>>();

        if parts.len() != 2 {
            return Err(());
        }

        let key = parts[0];
        let value = parts[1];

        match key {
            "SIG" | "sig" => Ok(ConnectionStringBean::SIG(value.to_string())),
            "EDT" | "edt" => {
                let datetime = match DateTime::parse_from_str(
                    value,
                    "%Y-%m-%dT%H:%M:%S%:z",
                ) {
                    Ok(datetime) => datetime
                        .with_timezone(&Local)
                        .with_nanosecond(0)
                        .expect("Invalid datetime"),
                    Err(err) => {
                        tracing::error!("Error parsing datetime: {}", err);
                        return Err(());
                    }
                };

                Ok(ConnectionStringBean::EDT(datetime))
            }
            "TID" | "tid" => {
                let tenant_id = Uuid::parse_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::TID(tenant_id))
            }
            "AID" | "aid" => {
                let account_id = Uuid::parse_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::AID(account_id))
            }
            "SID" | "sid" => {
                let subscription_account_id =
                    Uuid::parse_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::SID(subscription_account_id))
            }
            "RLS" | "rls" => {
                let roles = value
                    .split(',')
                    .map(|role| role.to_string())
                    .collect::<Vec<String>>();

                Ok(ConnectionStringBean::RLS(roles))
            }
            "PM" | "pm" => {
                let permission = Permission::from_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::PM(permission))
            }
            "PR" | "pr" => {
                let roles = value
                    .split(',')
                    .map(|role| {
                        let role_parts = role.split(':').collect::<Vec<&str>>();

                        if role_parts.len() != 2 {
                            return Err(());
                        }

                        let role = role_parts[0];
                        let permission = role_parts[1];

                        let permission = Permission::from_i32(
                            permission.parse::<i32>().map_err(|_| ())?,
                        );

                        Ok((role.to_string(), permission))
                    })
                    .collect::<Result<PermissionedRoles, ()>>()?;

                Ok(ConnectionStringBean::PR(roles))
            }
            "URL" | "url" => Ok(ConnectionStringBean::URL(value.to_string())),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::guest_role::Permission;
    use crate::domain::dtos::security_group::PermissionedRoles;

    #[test]
    fn test_to_string() {
        let signature = "signature".to_string();
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let role = "role".to_string();
        let permission = Permission::Read;
        let mut permissioned_roles = PermissionedRoles::new();
        permissioned_roles.push(("role1".to_string(), Permission::Read));
        permissioned_roles.push(("role1".to_string(), Permission::Write));
        permissioned_roles.push(("role2".to_string(), Permission::Read));

        let signature_bean = ConnectionStringBean::SIG(signature.clone());
        let tenant_id_bean = ConnectionStringBean::TID(tenant_id);
        let account_id_bean = ConnectionStringBean::AID(account_id);
        let role_bean = ConnectionStringBean::RLS(vec![role.clone()]);
        let permission_bean = ConnectionStringBean::PM(permission.to_owned());
        let permissioned_roles_bean =
            ConnectionStringBean::PR(permissioned_roles);

        assert_eq!(signature_bean.to_string(), format!("sig={}", signature));
        assert_eq!(
            tenant_id_bean.to_string(),
            format!("tid={}", tenant_id.to_string())
        );
        assert_eq!(
            account_id_bean.to_string(),
            format!("aid={}", account_id.to_string())
        );
        assert_eq!(role_bean.to_string(), format!("rls={}", role));
        assert_eq!(
            permission_bean.to_string(),
            format!("pm={}", permission.to_string())
        );

        let expected_permissioned_roles_string =
            format!("pr=role1:0,role1:1,role2:0");

        assert_eq!(
            permissioned_roles_bean.to_string(),
            expected_permissioned_roles_string
        );
    }

    #[test]
    fn test_try_from() {
        let signature = "signature".to_string();
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let role = "role".to_string();
        let permission = Permission::Read;
        let mut permissioned_roles = PermissionedRoles::new();
        permissioned_roles.push(("role1".to_string(), Permission::Read));
        permissioned_roles.push(("role1".to_string(), Permission::Write));
        permissioned_roles.push(("role2".to_string(), Permission::Read));

        let signature_bean = ConnectionStringBean::SIG(signature.clone());
        let tenant_id_bean = ConnectionStringBean::TID(tenant_id);
        let account_id_bean = ConnectionStringBean::AID(account_id);
        let role_bean = ConnectionStringBean::RLS(vec![role.clone()]);
        let permission_bean = ConnectionStringBean::PM(permission.to_owned());
        let permissioned_roles_bean =
            ConnectionStringBean::PR(permissioned_roles);

        assert_eq!(
            ConnectionStringBean::try_from(signature_bean.to_string()).unwrap(),
            signature_bean
        );
        assert_eq!(
            ConnectionStringBean::try_from(tenant_id_bean.to_string()).unwrap(),
            tenant_id_bean
        );
        assert_eq!(
            ConnectionStringBean::try_from(account_id_bean.to_string())
                .unwrap(),
            account_id_bean
        );
        assert_eq!(
            ConnectionStringBean::try_from(role_bean.to_string()).unwrap(),
            role_bean
        );
        assert_eq!(
            ConnectionStringBean::try_from(permission_bean.to_string())
                .unwrap(),
            permission_bean
        );
        assert_eq!(
            ConnectionStringBean::try_from(permissioned_roles_bean.to_string())
                .unwrap(),
            permissioned_roles_bean
        );
    }

    #[test]
    fn test_permissioned_roles() {
        let mut permissioned_roles = PermissionedRoles::new();
        permissioned_roles.push(("role1".to_string(), Permission::Read));
        permissioned_roles.push(("role1".to_string(), Permission::Write));
        permissioned_roles.push(("role2".to_string(), Permission::Read));

        let permissioned_roles_bean =
            ConnectionStringBean::PR(permissioned_roles);

        let permissioned_roles_string = permissioned_roles_bean.to_string();

        let parsed_permissioned_roles_bean =
            ConnectionStringBean::try_from(permissioned_roles_string).unwrap();

        assert_eq!(permissioned_roles_bean, parsed_permissioned_roles_bean);
    }
}
