//! Conversion helpers between domain types and their SQLite TEXT storage.
//!
//! SQLite (via Diesel) has no native `Uuid`, `Timestamptz`, `Jsonb` or `Array`
//! types, so those columns are stored as TEXT. These helpers centralize the
//! encode/decode so the repository layer round-trips values faithfully.

use chrono::{DateTime, NaiveDateTime, Utc};
use mycelium_base::utils::errors::{dto_err, MappedErrors};
use serde_json::Value;
use uuid::Uuid;

// ? ----------------------------------------------------------------------------
// ? UUID <-> TEXT
// ? ----------------------------------------------------------------------------

pub fn uuid_to_text(id: &Uuid) -> String {
    id.to_string()
}

pub fn uuid_from_text(value: &str) -> Result<Uuid, MappedErrors> {
    Uuid::parse_str(value)
        .map_err(|err| dto_err(format!("Invalid UUID in SQLite row: {err}")))
}

// ? ----------------------------------------------------------------------------
// ? TIMESTAMPTZ <-> TEXT (RFC3339, normalized to UTC)
// ? ----------------------------------------------------------------------------

pub fn timestamp_to_text(moment: &DateTime<Utc>) -> String {
    moment.to_rfc3339()
}

pub fn timestamp_from_text(value: &str) -> Result<DateTime<Utc>, MappedErrors> {
    let parsed = DateTime::parse_from_rfc3339(value).map_err(|err| {
        dto_err(format!("Invalid timestamp in SQLite row: {err}"))
    })?;

    Ok(parsed.with_timezone(&Utc))
}

// ? ----------------------------------------------------------------------------
// ? NaiveDateTime <-> TEXT
// ?
// ? The postgres repositories store `Local::now().naive_utc()` and read it back
// ? via `.and_local_timezone(Local).unwrap()` -- reinterpreting the naive UTC
// ? instant as if it were already in the local zone. That round-trip (not a true
// ? UTC->Local conversion) is preserved here so domain-visible timestamps behave
// ? identically between backends.
// ? ----------------------------------------------------------------------------

const NAIVE_TIMESTAMP_FORMAT: &str = "%Y-%m-%dT%H:%M:%S%.f";

pub fn naive_timestamp_to_text(moment: &NaiveDateTime) -> String {
    moment.format(NAIVE_TIMESTAMP_FORMAT).to_string()
}

pub fn naive_timestamp_from_text(
    value: &str,
) -> Result<NaiveDateTime, MappedErrors> {
    NaiveDateTime::parse_from_str(value, NAIVE_TIMESTAMP_FORMAT).map_err(
        |err| dto_err(format!("Invalid naive timestamp in SQLite row: {err}")),
    )
}

// ? ----------------------------------------------------------------------------
// ? JSONB <-> TEXT
// ? ----------------------------------------------------------------------------

pub fn json_to_text(value: &Value) -> Result<String, MappedErrors> {
    serde_json::to_string(value).map_err(|err| {
        dto_err(format!("Failed to serialize JSON for SQLite: {err}"))
    })
}

pub fn json_from_text(value: &str) -> Result<Value, MappedErrors> {
    serde_json::from_str(value)
        .map_err(|err| dto_err(format!("Invalid JSON in SQLite row: {err}")))
}

// ? ----------------------------------------------------------------------------
// ? ARRAY<TEXT> <-> TEXT (JSON array)
// ? ----------------------------------------------------------------------------

pub fn string_array_to_text(items: &[String]) -> Result<String, MappedErrors> {
    serde_json::to_string(items).map_err(|err| {
        dto_err(format!(
            "Failed to serialize string array for SQLite: {err}"
        ))
    })
}

pub fn string_array_from_text(
    value: &str,
) -> Result<Vec<String>, MappedErrors> {
    serde_json::from_str(value).map_err(|err| {
        dto_err(format!("Invalid string array in SQLite row: {err}"))
    })
}

// ? ----------------------------------------------------------------------------
// ? ARRAY<JSONB> <-> TEXT (JSON array of values)
// ? ----------------------------------------------------------------------------

pub fn json_array_to_text(items: &[Value]) -> Result<String, MappedErrors> {
    serde_json::to_string(items).map_err(|err| {
        dto_err(format!("Failed to serialize JSON array for SQLite: {err}"))
    })
}

pub fn json_array_from_text(value: &str) -> Result<Vec<Value>, MappedErrors> {
    serde_json::from_str(value).map_err(|err| {
        dto_err(format!("Invalid JSON array in SQLite row: {err}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn uuid_round_trips() {
        let id = Uuid::new_v4();
        assert_eq!(uuid_from_text(&uuid_to_text(&id)).unwrap(), id);
    }

    #[test]
    fn uuid_rejects_garbage() {
        assert!(uuid_from_text("not-a-uuid").is_err());
    }

    #[test]
    fn timestamp_round_trips_to_utc() {
        let now = Utc::now();
        let decoded = timestamp_from_text(&timestamp_to_text(&now)).unwrap();
        // RFC3339 preserves down to the encoded precision; compare timestamps.
        assert_eq!(decoded.timestamp_micros(), now.timestamp_micros());
    }

    #[test]
    fn naive_timestamp_round_trips() {
        let now = chrono::Local::now().naive_utc();
        let decoded =
            naive_timestamp_from_text(&naive_timestamp_to_text(&now)).unwrap();
        assert_eq!(decoded, now);
    }

    #[test]
    fn json_round_trips() {
        let value = json!({"a": 1, "b": ["x", null], "c": {"d": true}});
        assert_eq!(
            json_from_text(&json_to_text(&value).unwrap()).unwrap(),
            value
        );
    }

    #[test]
    fn string_array_round_trips_including_empty() {
        let items = vec!["read".to_string(), "write".to_string()];
        assert_eq!(
            string_array_from_text(&string_array_to_text(&items).unwrap())
                .unwrap(),
            items
        );

        let empty: Vec<String> = vec![];
        assert_eq!(
            string_array_from_text(&string_array_to_text(&empty).unwrap())
                .unwrap(),
            empty
        );
    }

    #[test]
    fn json_array_round_trips() {
        let items = vec![json!({"k": 1}), json!("v"), json!(null)];
        assert_eq!(
            json_array_from_text(&json_array_to_text(&items).unwrap()).unwrap(),
            items
        );
    }
}
