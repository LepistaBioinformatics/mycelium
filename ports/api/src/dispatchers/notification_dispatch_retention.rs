use chrono::{DateTime, Duration, Utc};
use myc_core::domain::entities::NotificationDispatchRegistration;

/// Minimum time between two purges of the notification dispatch trace.
const PURGE_INTERVAL_IN_HOURS: i64 = 1;

/// Retention of the notification dispatch trace, run from the email
/// dispatcher tick
///
/// Rows older than `retentionDays` are deleted at most once per hour, however
/// short the dispatcher interval is. A failed purge is logged and counts as
/// the run of its hour, so a broken database is not hammered on every tick;
/// it never stops the dispatcher loop.
pub(crate) struct NotificationDispatchRetention {
    retention_days: u32,
    last_purge: Option<DateTime<Utc>>,
}

impl NotificationDispatchRetention {
    pub(crate) fn new(retention_days: u32) -> Self {
        Self {
            retention_days,
            last_purge: None,
        }
    }

    /// Purge the expired rows when the last purge is an hour old or more
    #[tracing::instrument(name = "purge_notification_dispatches", skip_all)]
    pub(crate) async fn purge_if_due(
        &mut self,
        now: DateTime<Utc>,
        repository: &dyn NotificationDispatchRegistration,
    ) {
        if !self.is_due(now) {
            return;
        }

        self.last_purge = Some(now);

        let cutoff = now - Duration::days(i64::from(self.retention_days));

        match repository.purge_older_than(cutoff).await {
            Ok(0) => (),
            Ok(purged) => tracing::info!(
                "'{purged}' notification dispatch rows purged by retention"
            ),
            Err(err) => tracing::error!(
                "Notification dispatch retention purge failed: {err}"
            ),
        }
    }

    fn is_due(&self, now: DateTime<Utc>) -> bool {
        let Some(last_purge) = self.last_purge else {
            return true;
        };

        now - last_purge >= Duration::hours(PURGE_INTERVAL_IN_HOURS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use myc_core::domain::dtos::{
        message::MessageSendingEvent,
        notification_dispatch::{NewNotificationDispatch, QuotaOutcome},
    };
    use mycelium_base::utils::errors::{fetching_err, MappedErrors};
    use std::sync::Mutex;

    /// Records every cutoff it is asked to purge, failing when told to.
    #[derive(Default)]
    struct CountingRepository {
        cutoffs: Mutex<Vec<DateTime<Utc>>>,
        fails: bool,
    }

    #[async_trait]
    impl NotificationDispatchRegistration for CountingRepository {
        async fn register_and_enqueue(
            &self,
            _: &str,
            _: u32,
            _: Vec<NewNotificationDispatch>,
            _: Vec<MessageSendingEvent>,
        ) -> Result<QuotaOutcome, MappedErrors> {
            unreachable!()
        }

        async fn purge_older_than(
            &self,
            cutoff: DateTime<Utc>,
        ) -> Result<u64, MappedErrors> {
            self.cutoffs.lock().unwrap().push(cutoff);

            if self.fails {
                return fetching_err("database unavailable").as_error();
            }

            Ok(3)
        }
    }

    fn minutes(count: i64) -> Duration {
        Duration::minutes(count)
    }

    #[tokio::test]
    async fn purges_once_per_hour_of_ticks() {
        let repository = CountingRepository::default();
        let mut retention = NotificationDispatchRetention::new(90);
        let start = Utc::now();

        for tick in [0, 1, 30, 59] {
            retention
                .purge_if_due(start + minutes(tick), &repository)
                .await;
        }

        assert_eq!(repository.cutoffs.lock().unwrap().len(), 1);

        retention
            .purge_if_due(start + minutes(60), &repository)
            .await;
        retention
            .purge_if_due(start + minutes(61), &repository)
            .await;
        retention
            .purge_if_due(start + minutes(121), &repository)
            .await;

        let cutoffs = repository.cutoffs.lock().unwrap();

        assert_eq!(cutoffs.len(), 3);
        assert_eq!(cutoffs[0], start - Duration::days(90));
        assert_eq!(cutoffs[1], start + minutes(60) - Duration::days(90));
    }

    #[tokio::test]
    async fn a_failed_purge_is_survived_and_retried_next_hour() {
        let repository = CountingRepository {
            fails: true,
            ..Default::default()
        };
        let mut retention = NotificationDispatchRetention::new(7);
        let start = Utc::now();

        retention.purge_if_due(start, &repository).await;
        retention
            .purge_if_due(start + minutes(5), &repository)
            .await;
        retention
            .purge_if_due(start + minutes(65), &repository)
            .await;

        assert_eq!(repository.cutoffs.lock().unwrap().len(), 2);
    }
}
