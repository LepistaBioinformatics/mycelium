use chrono::{DateTime, Duration, Utc};

/// The span of the sliding window the hourly quota is counted over.
const WINDOW_SECS: i64 = 3600;

/// The notifications a sender dispatched during the last hour.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NotificationQuotaWindow {
    /// How many dispatches are inside the window.
    pub dispatched: u64,

    /// When the oldest dispatch inside the window was created.
    pub oldest: Option<DateTime<Utc>>,
}

impl NotificationQuotaWindow {
    /// The instant the window opens: only dispatches created strictly after
    /// it count against the quota
    pub fn opens_at(now: DateTime<Utc>) -> DateTime<Utc> {
        now - Duration::seconds(WINDOW_SECS)
    }

    /// Seconds a batch must wait, or `None` when the batch fits the quota
    ///
    /// A batch fits while `dispatched + batch_size <= quota_per_hour`, so a
    /// batch landing exactly on the quota is accepted and one more dispatch
    /// is refused. A batch is never split.
    pub fn retry_after_secs_if_exceeded(
        &self,
        quota_per_hour: u32,
        batch_size: usize,
        now: DateTime<Utc>,
    ) -> Option<u64> {
        let batch_size = u64::try_from(batch_size).unwrap_or(u64::MAX);
        let requested = self.dispatched.saturating_add(batch_size);

        if requested <= u64::from(quota_per_hour) {
            return None;
        }

        Some(self.secs_until_oldest_leaves(now))
    }

    /// Time until the oldest dispatch leaves the window, rounded up and kept
    /// within `1..=3600`
    ///
    /// An empty window means the batch alone is larger than the quota: the
    /// full window span is returned.
    fn secs_until_oldest_leaves(&self, now: DateTime<Utc>) -> u64 {
        let Some(oldest) = self.oldest else {
            return WINDOW_SECS.unsigned_abs();
        };

        let remaining = oldest + Duration::seconds(WINDOW_SECS) - now;
        let rounded_up = (remaining.num_milliseconds() + 999).div_euclid(1000);

        rounded_up.clamp(1, WINDOW_SECS).unsigned_abs()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    fn window(
        dispatched: u64,
        oldest_age_secs: i64,
    ) -> NotificationQuotaWindow {
        NotificationQuotaWindow {
            dispatched,
            oldest: Some(now() - Duration::seconds(oldest_age_secs)),
        }
    }

    #[test]
    fn window_opens_one_hour_before_now() {
        assert_eq!(
            NotificationQuotaWindow::opens_at(now()),
            now() - Duration::hours(1)
        );
    }

    #[test]
    fn batch_landing_exactly_on_the_quota_is_accepted() {
        let window = window(7, 10);

        assert_eq!(window.retry_after_secs_if_exceeded(10, 3, now()), None);
    }

    #[test]
    fn batch_one_over_the_quota_is_refused() {
        let window = window(7, 10);

        assert_eq!(
            window.retry_after_secs_if_exceeded(10, 4, now()),
            Some(3590)
        );
    }

    #[test]
    fn full_window_refuses_a_single_dispatch() {
        let window = window(10, 1800);

        assert_eq!(
            window.retry_after_secs_if_exceeded(10, 1, now()),
            Some(1800)
        );
    }

    #[test]
    fn retry_after_rounds_up_to_the_next_second() {
        let window = NotificationQuotaWindow {
            dispatched: 1,
            oldest: Some(now() - Duration::milliseconds(3_599_500)),
        };

        assert_eq!(window.retry_after_secs_if_exceeded(1, 1, now()), Some(1));
    }

    #[test]
    fn retry_after_is_never_zero_nor_negative() {
        for oldest_age_secs in [3600, 3601, 7200] {
            let window = window(1, oldest_age_secs);

            assert_eq!(
                window.retry_after_secs_if_exceeded(1, 1, now()),
                Some(1),
                "age {oldest_age_secs}"
            );
        }
    }

    #[test]
    fn retry_after_is_capped_when_the_oldest_row_is_in_the_future() {
        let window = window(1, -120);

        assert_eq!(
            window.retry_after_secs_if_exceeded(1, 1, now()),
            Some(3600)
        );
    }

    #[test]
    fn batch_larger_than_the_quota_waits_a_full_window() {
        let empty = NotificationQuotaWindow {
            dispatched: 0,
            oldest: None,
        };

        assert_eq!(empty.retry_after_secs_if_exceeded(2, 3, now()), Some(3600));
        assert_eq!(empty.retry_after_secs_if_exceeded(2, 2, now()), None);
        assert_eq!(empty.retry_after_secs_if_exceeded(2, 0, now()), None);
    }
}
