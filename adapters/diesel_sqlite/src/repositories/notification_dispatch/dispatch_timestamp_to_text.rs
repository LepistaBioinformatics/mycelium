use chrono::{DateTime, SecondsFormat, Utc};

/// Encode a `notification_dispatch.created_at` value
///
/// Always `YYYY-MM-DDTHH:MM:SS.ffffffZ`: a fixed width in UTC, so the text
/// order of two values is their time order. The quota window and the
/// retention purge compare this column as text, which the variable-width
/// `types::timestamp_to_text` form does not guarantee.
pub(super) fn dispatch_timestamp_to_text(moment: &DateTime<Utc>) -> String {
    moment.to_rfc3339_opts(SecondsFormat::Micros, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::timestamp_from_text;

    use chrono::Duration;

    fn moment(micros: i64) -> DateTime<Utc> {
        DateTime::from_timestamp_micros(micros).unwrap()
    }

    #[test]
    fn text_has_a_fixed_width_whatever_the_fraction() {
        let whole_second =
            dispatch_timestamp_to_text(&moment(1_800_000_000_000_000));
        let with_micros =
            dispatch_timestamp_to_text(&moment(1_800_000_000_000_001));

        assert_eq!(whole_second, "2027-01-15T08:00:00.000000Z");
        assert_eq!(whole_second.len(), with_micros.len());
    }

    #[test]
    fn text_order_is_time_order() {
        let start = moment(1_800_000_000_000_000);
        let moments = [
            start - Duration::hours(1),
            start - Duration::microseconds(1),
            start,
            start + Duration::microseconds(1),
            start + Duration::milliseconds(500),
            start + Duration::seconds(1),
            start + Duration::days(40),
        ];

        let texts: Vec<String> =
            moments.iter().map(dispatch_timestamp_to_text).collect();
        let mut sorted = texts.clone();
        sorted.sort();

        assert_eq!(texts, sorted);
    }

    #[test]
    fn text_round_trips() {
        let original = moment(1_800_000_000_123_456);
        let text = dispatch_timestamp_to_text(&original);

        assert_eq!(timestamp_from_text(&text).unwrap(), original);
    }
}
