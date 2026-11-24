use crate::domain::dtos::federation::PeerKey;

use chrono::{DateTime, Local};
use std::time::Duration;

/// Whether an unknown `kid` may trigger a fresh fetch of the peer's keys
///
/// A peer that rotated its key signs with a `kid` this instance has not
/// stored yet; the keys are re-fetched once before the request is refused
/// (spec failure case 12). The fetch is allowed only when both the newest
/// stored key and the last handshake attempt (`last_attempt`, the peer's
/// `updated`) are more than `cool_down` old. The check runs before the
/// signature is verified, so without the attempt time anyone naming a peer
/// with no stored keys could make this instance call it on every request.
///
pub fn peer_keys_refresh_due(
    keys: &[PeerKey],
    last_attempt: Option<DateTime<Local>>,
    now: DateTime<Local>,
    cool_down: Duration,
) -> bool {
    let newest = keys
        .iter()
        .map(|key| key.fetched_at)
        .chain(last_attempt)
        .max();
    let Some(newest) = newest else {
        return true;
    };

    (now - newest).to_std().unwrap_or_default() >= cool_down
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;
    use uuid::Uuid;

    fn key(fetched_ago: i64) -> PeerKey {
        PeerKey {
            peer_id: Uuid::nil(),
            kid: "1".to_string(),
            alg: "EdDSA".to_string(),
            public_key: "x".to_string(),
            fetched_at: Local::now() - ChronoDuration::seconds(fetched_ago),
            expires_at: None,
        }
    }

    fn ago(seconds: i64) -> Option<DateTime<Local>> {
        Some(Local::now() - ChronoDuration::seconds(seconds))
    }

    #[test]
    fn refresh_table() {
        let cool_down = Duration::from_secs(30);
        let table: [(Vec<PeerKey>, Option<DateTime<Local>>, bool); 6] = [
            (vec![], None, true),
            (vec![], ago(5), false),
            (vec![], ago(60), true),
            (vec![key(5)], ago(60), false),
            (vec![key(60)], ago(60), true),
            (vec![key(60), key(5)], None, false),
        ];

        for (keys, last_attempt, expected) in table {
            assert_eq!(
                peer_keys_refresh_due(
                    &keys,
                    last_attempt,
                    Local::now(),
                    cool_down
                ),
                expected,
                "{} keys, last attempt {last_attempt:?}",
                keys.len()
            );
        }
    }
}
