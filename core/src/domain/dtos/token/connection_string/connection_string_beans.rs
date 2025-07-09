use crate::domain::dtos::{
    guest_role::Permission, security_group::PermissionedRole,
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

    /// A list of roles with your permissions
    RLS(Vec<PermissionedRole>),

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
                    .fold(String::new(), |acc, role| {
                        format!(
                            "{}{}:{},",
                            acc,
                            role.slug,
                            role.permission
                                .clone()
                                .unwrap_or_default()
                                .to_i32()
                        )
                    })
                    .trim_end_matches(',')
                    .to_string();

                format!("rls={}", roles)
            }
            //ConnectionStringBean::PM(permission) => {
            //    format!("pm={}", permission.to_string())
            //}
            //ConnectionStringBean::PR(permissioned_roles) => {
            //    let roles = permissioned_roles
            //        .iter()
            //        .fold(String::new(), |acc, (role, permission)| {
            //            format!("{}{}:{},", acc, role, permission.to_i32())
            //        })
            //        .trim_end_matches(',')
            //        .to_string();
            //    format!("pr={}", roles)
            //}
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

                        Ok(PermissionedRole {
                            slug: role.to_string(),
                            permission: Some(permission),
                        })
                    })
                    .collect::<Result<Vec<PermissionedRole>, ()>>()?;

                Ok(ConnectionStringBean::RLS(roles))
            }
            //"PM" | "pm" => {
            //    let permission = Permission::from_str(value).map_err(|_| ())?;
            //    Ok(ConnectionStringBean::PM(permission))
            //}
            //"PR" | "pr" => {
            //    let roles = value
            //        .split(',')
            //        .map(|role| {
            //            let role_parts = role.split(':').collect::<Vec<&str>>();
            //            if role_parts.len() != 2 {
            //                return Err(());
            //            }
            //            let role = role_parts[0];
            //            let permission = role_parts[1];
            //            let permission = Permission::from_i32(
            //                permission.parse::<i32>().map_err(|_| ())?,
            //            );
            //            Ok((role.to_string(), permission))
            //        })
            //        .collect::<Result<PermissionedRoles, ()>>()?;
            //    Ok(ConnectionStringBean::PR(roles))
            //}
            "URL" | "url" => Ok(ConnectionStringBean::URL(value.to_string())),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::dtos::guest_role::Permission;

    fn generate_roles() -> Vec<PermissionedRole> {
        vec![
            PermissionedRole {
                slug: "role1".to_string(),
                permission: Some(Permission::Read),
            },
            PermissionedRole {
                slug: "role1".to_string(),
                permission: Some(Permission::Write),
            },
            PermissionedRole {
                slug: "role2".to_string(),
                permission: Some(Permission::Read),
            },
            PermissionedRole {
                slug: "role3".to_string(),
                permission: None,
            },
        ]
    }

    #[test]
    fn test_to_string() {
        let signature = "signature".to_string();
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let roles = generate_roles();

        let signature_bean = ConnectionStringBean::SIG(signature.clone());
        let tenant_id_bean = ConnectionStringBean::TID(tenant_id);
        let account_id_bean = ConnectionStringBean::AID(account_id);
        let role_bean = ConnectionStringBean::RLS(roles.clone());

        assert_eq!(signature_bean.to_string(), format!("sig={}", signature));

        assert_eq!(
            tenant_id_bean.to_string(),
            format!("tid={}", tenant_id.to_string())
        );

        assert_eq!(
            account_id_bean.to_string(),
            format!("aid={}", account_id.to_string())
        );

        assert_eq!(
            role_bean.to_string(),
            format!(
                "rls={}",
                roles
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<String>>()
                    .join(",")
            )
        );
    }

    #[test]
    fn test_try_from() {
        let signature = "signature".to_string();
        let tenant_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let roles = generate_roles();

        let signature_bean = ConnectionStringBean::SIG(signature.clone());
        let tenant_id_bean = ConnectionStringBean::TID(tenant_id);
        let account_id_bean = ConnectionStringBean::AID(account_id);
        let role_bean = ConnectionStringBean::RLS(roles.clone());

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

        let url = "https://example.com";
        let url_bean = ConnectionStringBean::URL(url.to_string());

        assert_eq!(
            ConnectionStringBean::try_from(url_bean.to_string()).unwrap(),
            url_bean
        );
    }
}
