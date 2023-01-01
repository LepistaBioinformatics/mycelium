use chrono::{DateTime, Local};
use redis::{from_redis_value, ErrorKind, FromRedisValue, RedisResult, Value};
use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub(super) struct TmpToken {
    #[allow(dead_code)]
    pub expires: Option<DateTime<Local>>,

    #[allow(dead_code)]
    pub own_service: String,
}

impl FromRedisValue for TmpToken {
    fn from_redis_value(v: &Value) -> RedisResult<Self> {
        let json_str: String = from_redis_value(v)?;

        let result: Self = match serde_json::from_str::<Self>(&json_str) {
            Ok(v) => v,
            Err(_err) => {
                return Err(
                    (ErrorKind::TypeError, "Parse to JSON Failed").into()
                )
            }
        };

        Ok(result)
    }
}
