use std::str::FromStr;

use chrono::Local;
use clean_base::utils::errors::{execution_err, MappedErrors};
use log::warn;
use myc_core::domain::dtos::token::TokenDTO;
use regex::Regex;
use uuid::Uuid;

/// Build the default key to store redis records.
///
/// Default keys concatenates the token UUID and the requesting service name.
/// The result string should be used as the Redis storing key.
pub(super) fn to_redis_key(token: TokenDTO) -> String {
    format!("{}-{}", token.token.to_string(), token.own_service)
}

/// Extract elements from the redis key.
///
/// The refereed elements contains the token uuid and the own_service that
/// composes the TokenDTO.
pub(super) fn from_redis_key(
    key: String,
) -> Result<(Uuid, String), MappedErrors> {
    let pat = match Regex::new(
        r"^([0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})-(.+$)",
    ) {
        Err(err) => {
            return Err(execution_err(
                format!("Could not parse redis key: {err}"),
                None,
                None,
            ))
        }
        Ok(res) => res,
    };

    match pat.captures(key.as_str()) {
        None => {
            return Err(execution_err(
                String::from("Could not parse redis key: unable to match id"),
                None,
                None,
            ));
        }
        Some(res) => {
            if res.len() != 3 {
                return Err(execution_err(
                    String::from(
                        "Could not parse redis key: insufficient arguments to 
                        be parsed.",
                    ),
                    None,
                    None,
                ));
            }

            let id = match Uuid::from_str(&res[1].to_string()) {
                Err(err) => {
                    warn!("err: {:?}", err);
                    return Err(execution_err(
                        format!("Could not parse redis key: {err}"),
                        None,
                        None,
                    ));
                }
                Ok(res) => res,
            };

            Ok((id, res[2].to_string()))
        }
    }
}

/// Build the current day Redis key.
///
/// Such key are used to segment keys by day. This segmentation strategy allows
/// the system to remove old keys not consumed by the system.
pub(super) fn get_today_key() -> String {
    format!("tokens-{}", Local::now().date_naive().to_string())
}
