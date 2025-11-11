use crate::domain::dtos::{
    guest_role::Permission, native_error_codes::NativeErrorCodes,
};

use base64::{engine::general_purpose, Engine};
use mycelium_base::utils::errors::{execution_err, MappedErrors};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub struct LicensedResource {
    /// The guest account unique id
    ///
    /// This is the unique identifier of the account that is own of the
    /// resource to be managed.
    #[serde(alias = "guest_account_id")]
    pub acc_id: Uuid,

    /// If the guest account is a system account
    ///
    /// System accounts has permissions to act as special users into the
    /// Mycelium system.
    #[serde(alias = "guest_account_is_default")]
    pub sys_acc: bool,

    /// The guest account tenant unique id
    ///
    /// This is the unique identifier of the tenant that is own of the resource
    /// to be managed.
    pub tenant_id: Uuid,

    /// The guest account name
    ///
    /// This is the name of the account that is own of the resource to be
    /// managed.
    #[serde(alias = "guest_account_name")]
    pub acc_name: String,

    /// The guest account role verbose name
    ///
    /// This is the verbose name of the role that is own of the resource to be
    /// managed.
    pub role: String,

    /// The Guest Role ID
    ///
    /// This is the ID of the guest role that is own of the resource to be
    /// managed.
    pub role_id: Uuid,

    /// The guest role permissions
    ///
    /// This is the list of permissions that the guest role has.
    #[serde(alias = "permission")]
    pub perm: Permission,

    /// If the guest account was verified
    ///
    /// If the user accepted the invitation to join the account, the account
    /// should be verified.
    ///
    pub verified: bool,
}

impl LicensedResource {
    pub(super) fn is_uuid(value: &str) -> bool {
        let uuid_format = vec![8, 4, 4, 4, 12];
        let mut chars = value.chars().peekable();

        for &count in &uuid_format {
            for _ in 0..count {
                match chars.next() {
                    Some(c) if c.is_ascii_hexdigit() => continue,
                    _ => return false,
                }
            }

            if let Some('-') = chars.peek() {
                chars.next();
            }
        }
        chars.next().is_none()
    }

    /// Try to load a UUID v4 hex as UUID from a string
    ///
    pub fn load_uuid(value: String) -> Result<Uuid, MappedErrors> {
        match Uuid::from_str(&value) {
            Ok(uuid) => Ok(uuid),
            Err(_) => execution_err(format!("Invalid UUID: {}", value))
                .with_code(NativeErrorCodes::MYC00019)
                .with_exp_true()
                .as_error(),
        }
    }
}

impl ToString for LicensedResource {
    fn to_string(&self) -> String {
        //
        // Encode account name as base64
        //
        let encoded_account_name =
            general_purpose::STANDARD.encode(self.acc_name.as_bytes());

        format!(
            "t/{tenant_id}/a/{acc_id}/r/{role_id}?p={role}:{perm}&s={is_acc_std}&v={verified}&n={acc_name}",
            tenant_id = self.tenant_id.to_string().replace("-", ""),
            acc_id = self.acc_id.to_string().replace("-", ""),
            role_id = self.role_id.to_string().replace("-", ""),
            role = self.role,
            perm = self.perm.to_owned().to_i32(),
            is_acc_std = self.sys_acc as i8,
            verified = self.verified as i8,
            acc_name = encoded_account_name,
        )
    }
}

impl FromStr for LicensedResource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let full_url = format!("https://localhost.local/{s}");

        let url = Url::from_str(&full_url).map_err(|e| {
            format!("Unexpected error on check license URL: {:?}", e)
        })?;

        //
        // Extract the path segments
        //
        let segments: Vec<_> =
            url.path_segments().ok_or("Path not found")?.collect();

        if segments.len() != 6
            || segments[0] != "t"
            || segments[2] != "a"
            || segments[4] != "r"
        {
            return Err("Invalid path format".to_string());
        }

        let tenant_id = segments[1];
        let account_id = segments[3];
        let role_id = segments[5];

        if !Self::is_uuid(tenant_id) {
            return Err("Invalid tenant UUID".to_string());
        }

        if !Self::is_uuid(account_id) {
            return Err("Invalid account UUID".to_string());
        }

        if !Self::is_uuid(role_id) {
            return Err("Invalid role UUID".to_string());
        }

        //
        // Extract the query parameters
        //
        let permissioned_role = url
            .query_pairs()
            .find(|(key, _)| key == "p")
            .map(|(_, value)| value)
            .ok_or("Parameter permissions not found")?;

        let permissioned_role: Vec<_> = permissioned_role.split(':').collect();

        if permissioned_role.len() != 2 {
            return Err("Invalid permissioned role format".to_string());
        }

        let role_name = permissioned_role[0];
        let permission_code = permissioned_role[1];

        let sys = match url
            .query_pairs()
            .find(|(key, _)| key == "s")
            .map(|(_, value)| value)
            .ok_or("Parameter sys not found")?
            .parse::<i8>()
        {
            Ok(sys) => match sys {
                0 => false,
                1 => true,
                _ => {
                    return Err("Invalid account standard".to_string());
                }
            },
            Err(_) => {
                return Err("Failed to parse account standard".to_string());
            }
        };

        let verified = match url
            .query_pairs()
            .find(|(key, _)| key == "v")
            .map(|(_, value)| value)
            .ok_or("Parameter v not found")?
            .parse::<i8>()
        {
            Ok(verified) => match verified {
                0 => false,
                1 => true,
                _ => {
                    return Err("Invalid account verification".to_string());
                }
            },
            Err(_) => {
                return Err("Failed to parse account verification".to_string());
            }
        };

        let name_encoded = url
            .query_pairs()
            .find(|(key, _)| key == "n")
            .map(|(_, value)| value)
            .ok_or("Parameter name not found")?;

        let name_decoded =
            match general_purpose::STANDARD.decode(name_encoded.as_bytes()) {
                Ok(name) => name,
                Err(_) => {
                    return Err("Failed to decode account name".to_string());
                }
            };

        Ok(Self {
            tenant_id: Uuid::from_str(tenant_id).unwrap(),
            acc_id: Uuid::from_str(account_id).unwrap(),
            role_id: Uuid::from_str(role_id).unwrap(),
            role: role_name.to_string(),
            perm: Permission::from_i32(permission_code.parse::<i32>().unwrap()),
            sys_acc: sys,
            acc_name: String::from_utf8(name_decoded).unwrap(),
            verified,
        })
    }
}

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum LicensedResources {
    Records(Vec<LicensedResource>),
    Urls(Vec<String>),
}

impl LicensedResources {
    pub fn to_licenses_vector(&self) -> Vec<LicensedResource> {
        match self {
            Self::Records(records) => records.to_owned(),
            Self::Urls(urls) => urls
                .iter()
                .map(|i| LicensedResource::from_str(i).unwrap())
                .collect(),
        }
    }
}
