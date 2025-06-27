use mycelium_base::utils::errors::{invalid_arg_err, MappedErrors};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};
use utoipa::ToSchema;

#[derive(Clone, Debug, Deserialize, Serialize, ToSchema, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub enum VerboseStatus {
    Unverified,
    Verified,
    Inactive,
    Archived,
    Deleted,
    Unknown,
}

impl FromStr for VerboseStatus {
    type Err = VerboseStatus;

    fn from_str(s: &str) -> Result<VerboseStatus, VerboseStatus> {
        match s {
            "unverified" => Ok(VerboseStatus::Unverified),
            "verified" => Ok(VerboseStatus::Verified),
            "inactive" => Ok(VerboseStatus::Inactive),
            "archived" => Ok(VerboseStatus::Archived),
            "deleted" => Ok(VerboseStatus::Deleted),
            _ => Err(VerboseStatus::Unknown),
        }
    }
}

impl Display for VerboseStatus {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        match self {
            VerboseStatus::Unverified => write!(f, "unverified"),
            VerboseStatus::Verified => write!(f, "verified"),
            VerboseStatus::Inactive => write!(f, "inactive"),
            VerboseStatus::Archived => write!(f, "archived"),
            VerboseStatus::Deleted => write!(f, "deleted"),
            VerboseStatus::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct FlagResponse {
    pub is_active: Option<bool>,
    pub is_checked: Option<bool>,
    pub is_archived: Option<bool>,
    pub is_deleted: Option<bool>,
}

impl VerboseStatus {
    pub fn from_flags(
        is_active: bool,
        is_checked: bool,
        is_archived: bool,
        is_deleted: bool,
    ) -> Self {
        if is_deleted == true {
            return VerboseStatus::Deleted;
        }

        if is_active == false {
            return VerboseStatus::Inactive;
        }

        if is_checked == false {
            return VerboseStatus::Unverified;
        }

        if is_archived == true {
            return VerboseStatus::Archived;
        }

        if is_archived == false {
            return VerboseStatus::Verified;
        }

        VerboseStatus::Unknown
    }

    pub fn to_flags(&self) -> Result<FlagResponse, MappedErrors> {
        match self {
            VerboseStatus::Inactive => Ok(FlagResponse {
                is_active: Some(false),
                is_checked: None,
                is_archived: None,
                is_deleted: None,
            }),
            VerboseStatus::Unverified => Ok(FlagResponse {
                is_active: Some(true),
                is_checked: Some(false),
                is_archived: None,
                is_deleted: None,
            }),
            VerboseStatus::Archived => Ok(FlagResponse {
                is_active: Some(true),
                is_checked: Some(true),
                is_archived: Some(true),
                is_deleted: None,
            }),
            VerboseStatus::Verified => Ok(FlagResponse {
                is_active: Some(true),
                is_checked: Some(true),
                is_archived: Some(false),
                is_deleted: None,
            }),
            VerboseStatus::Deleted => Ok(FlagResponse {
                is_active: Some(false),
                is_checked: Some(true),
                is_archived: None,
                is_deleted: Some(true),
            }),
            VerboseStatus::Unknown => invalid_arg_err(
                "Account status could not be `Unknown`".to_string(),
            )
            .as_error(),
        }
    }
}
