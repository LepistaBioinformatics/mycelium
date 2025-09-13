use super::LicensedResource;

use base64::{engine::general_purpose, Engine};
use chrono::{DateTime, Local};
use reqwest::Url;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use utoipa::{ToResponse, ToSchema};
use uuid::Uuid;

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq, ToResponse,
)]
pub struct TenantOwnership {
    /// The tenant ID that the profile has administration privileges
    pub id: Uuid,

    /// The tenant name
    pub name: String,

    /// The date and time the tenant was granted to the profile
    pub since: DateTime<Local>,
}

#[derive(
    Clone, Debug, Deserialize, Serialize, ToSchema, PartialEq, ToResponse,
)]
#[serde(rename_all = "camelCase")]
pub enum TenantsOwnership {
    Records(Vec<TenantOwnership>),
    Urls(Vec<String>),
}

impl ToString for TenantOwnership {
    fn to_string(&self) -> String {
        let encoded_name =
            general_purpose::STANDARD.encode(self.name.as_bytes());

        format!(
            "tid/{tenant_id}?since={since}&name={name}",
            tenant_id = self.id.to_string().replace("-", ""),
            since = self.since.to_rfc3339(),
            name = encoded_name,
        )
    }
}

impl FromStr for TenantOwnership {
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

        if segments.len() != 2 || segments[0] != "tid" {
            return Err("Invalid path format".to_string());
        }

        let tenant_id = segments[1];

        if !LicensedResource::is_uuid(tenant_id) {
            return Err("Invalid tenant UUID".to_string());
        }

        //
        // Extract the since
        //
        let since = match url
            .query_pairs()
            .find(|(key, _)| key == "since")
            .map(|(_, value)| value)
            .ok_or("Parameter since not found")?
            .parse::<DateTime<Local>>()
        {
            Ok(since) => since,
            Err(_) => {
                return Err(
                    "Failed to parse tenant ownership since".to_string()
                );
            }
        };

        //
        // Extract the name
        //
        let name_encoded = match url
            .query_pairs()
            .find(|(key, _)| key == "name")
            .map(|(_, value)| value)
            .ok_or("Parameter name not found")?
            .parse::<String>()
        {
            Ok(name) => name,
            Err(_) => {
                return Err("Failed to parse tenant ownership name".to_string());
            }
        };

        let name_decoded =
            match general_purpose::STANDARD.decode(name_encoded.as_bytes()) {
                Ok(name) => String::from_utf8(name).unwrap(),
                Err(_) => {
                    return Err(
                        "Failed to decode tenant ownership name".to_string()
                    );
                }
            };

        Ok(Self {
            id: Uuid::from_str(tenant_id).unwrap(),
            name: name_decoded,
            since,
        })
    }
}

impl TenantsOwnership {
    pub fn to_ownership_vector(&self) -> Vec<TenantOwnership> {
        match self {
            Self::Records(records) => records.to_owned(),
            Self::Urls(urls) => urls
                .iter()
                .map(|i| TenantOwnership::from_str(i).unwrap())
                .collect(),
        }
    }
}
