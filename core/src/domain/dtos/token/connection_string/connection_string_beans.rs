use crate::domain::dtos::{
    guest_role::Permission, route_type::PermissionedRoles,
};

use chrono::{DateTime, Local, Timelike};
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum ConnectionStringBean {
    Signature(String),
    ExpirationDateTime(DateTime<Local>),
    TenantId(Uuid),
    AccountId(Uuid),
    Role(String),
    Permission(Permission),
    PermissionedRoles(PermissionedRoles),
    Endpoint(String),
}

impl ToString for ConnectionStringBean {
    fn to_string(&self) -> String {
        match self {
            ConnectionStringBean::Signature(signature) => {
                format!("Signature={}", signature)
            }
            ConnectionStringBean::ExpirationDateTime(expiration_date) => {
                format!(
                    "ExpirationDateTime={}",
                    expiration_date.format("%Y-%m-%dT%H:%M:%S%:z").to_string()
                )
            }
            ConnectionStringBean::TenantId(tenant_id) => {
                format!("TenantId={}", tenant_id.to_string())
            }
            ConnectionStringBean::AccountId(account_id) => {
                format!("AccountId={}", account_id.to_string())
            }
            ConnectionStringBean::Role(role) => {
                format!("Role={}", role)
            }
            ConnectionStringBean::Permission(permission) => {
                format!("Permission={}", permission.to_string())
            }
            ConnectionStringBean::PermissionedRoles(permissioned_roles) => {
                let roles = permissioned_roles
                    .iter()
                    .fold(String::new(), |acc, (role, permission)| {
                        format!("{}{}:{},", acc, role, permission.to_i32())
                    })
                    .trim_end_matches(',')
                    .to_string();

                format!("PermissionedRoles={}", roles)
            }
            ConnectionStringBean::Endpoint(endpoint) => {
                format!("Endpoint={}", endpoint)
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
            "Signature" => {
                Ok(ConnectionStringBean::Signature(value.to_string()))
            }
            "ExpirationDateTime" => {
                let datetime = match DateTime::parse_from_str(
                    value,
                    "%Y-%m-%dT%H:%M:%S%:z",
                ) {
                    Ok(datetime) => datetime
                        .with_timezone(&Local)
                        .with_nanosecond(0)
                        .expect("Invalid datetime"),
                    Err(err) => {
                        eprintln!("Error parsing datetime: {}", err);
                        return Err(());
                    }
                };

                Ok(ConnectionStringBean::ExpirationDateTime(datetime))
            }
            "TenantId" => {
                let tenant_id = Uuid::parse_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::TenantId(tenant_id))
            }
            "AccountId" => {
                let account_id = Uuid::parse_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::AccountId(account_id))
            }
            "Role" => Ok(ConnectionStringBean::Role(value.to_string())),
            "Permission" => {
                let permission = Permission::from_str(value).map_err(|_| ())?;
                Ok(ConnectionStringBean::Permission(permission))
            }
            "PermissionedRoles" => {
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

                Ok(ConnectionStringBean::PermissionedRoles(roles))
            }
            "Endpoint" => Ok(ConnectionStringBean::Endpoint(value.to_string())),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::guest_role::Permission;
    use crate::domain::dtos::route_type::PermissionedRoles;

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

        let signature_bean = ConnectionStringBean::Signature(signature.clone());
        let tenant_id_bean = ConnectionStringBean::TenantId(tenant_id);
        let account_id_bean = ConnectionStringBean::AccountId(account_id);
        let role_bean = ConnectionStringBean::Role(role.clone());
        let permission_bean =
            ConnectionStringBean::Permission(permission.to_owned());
        let permissioned_roles_bean =
            ConnectionStringBean::PermissionedRoles(permissioned_roles);

        assert_eq!(
            signature_bean.to_string(),
            format!("Signature={}", signature)
        );
        assert_eq!(
            tenant_id_bean.to_string(),
            format!("TenantId={}", tenant_id.to_string())
        );
        assert_eq!(
            account_id_bean.to_string(),
            format!("AccountId={}", account_id.to_string())
        );
        assert_eq!(role_bean.to_string(), format!("Role={}", role));
        assert_eq!(
            permission_bean.to_string(),
            format!("Permission={}", permission.to_string())
        );

        let expected_permissioned_roles_string =
            format!("PermissionedRoles=role1:0,role1:1,role2:0");

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

        let signature_bean = ConnectionStringBean::Signature(signature.clone());
        let tenant_id_bean = ConnectionStringBean::TenantId(tenant_id);
        let account_id_bean = ConnectionStringBean::AccountId(account_id);
        let role_bean = ConnectionStringBean::Role(role.clone());
        let permission_bean =
            ConnectionStringBean::Permission(permission.to_owned());
        let permissioned_roles_bean =
            ConnectionStringBean::PermissionedRoles(permissioned_roles);

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
            ConnectionStringBean::PermissionedRoles(permissioned_roles);

        println!("1) {:?}", permissioned_roles_bean);

        let permissioned_roles_string = permissioned_roles_bean.to_string();

        println!("2) {}", permissioned_roles_string);

        let parsed_permissioned_roles_bean =
            ConnectionStringBean::try_from(permissioned_roles_string).unwrap();

        println!("3) {:?}", parsed_permissioned_roles_bean);

        assert_eq!(permissioned_roles_bean, parsed_permissioned_roles_bean);
    }
}
