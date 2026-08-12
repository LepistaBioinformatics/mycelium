use myc_core::domain::dtos::written_by::WrittenBy;
use serde_json::{from_value, json, Value};

pub(super) fn parse_optional_written_by(
    written_by: Option<Value>,
) -> Option<WrittenBy> {
    written_by
        .map(|m| {
            if m == json!({}) {
                None
            } else {
                let modifier: WrittenBy =
                    from_value(m).unwrap_or(WrittenBy::default());

                Some(modifier)
            }
        })
        .flatten()
}
