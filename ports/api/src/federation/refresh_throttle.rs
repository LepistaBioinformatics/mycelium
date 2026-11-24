use std::{
    collections::HashMap,
    sync::Mutex,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// At most one background refresh per subject per window, whatever its
/// outcome, in this process
///
/// A subject the issuer answers "unknown" for has nothing stored, and
/// "unknown" never opens the circuit: without this, every request of that
/// subject would call the peer again.
///
#[derive(Default)]
pub(crate) struct RefreshThrottle(Mutex<HashMap<Uuid, Instant>>);

impl RefreshThrottle {
    /// Whether a refresh may start now; if so, it counts as the last one
    pub(crate) fn admit(&self, subject: Uuid, window: Duration) -> bool {
        let Ok(mut last) = self.0.lock() else {
            return false;
        };
        let now = Instant::now();
        last.retain(|_, at| now.duration_since(*at) < window);

        if last.contains_key(&subject) {
            return false;
        }

        last.insert(subject, now);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_refresh_per_subject_per_window() {
        let throttle = RefreshThrottle::default();
        let (subject, other) = (Uuid::new_v4(), Uuid::new_v4());
        let window = Duration::from_secs(30);

        let admitted =
            (0..10).filter(|_| throttle.admit(subject, window)).count();

        assert_eq!(admitted, 1);
        assert!(throttle.admit(other, window));
        assert!(throttle.admit(subject, Duration::ZERO));
    }
}
