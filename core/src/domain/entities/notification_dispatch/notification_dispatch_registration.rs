use crate::domain::dtos::{
    message::MessageSendingEvent,
    notification_dispatch::{NewNotificationDispatch, QuotaOutcome},
};

use async_trait::async_trait;
use chrono::{DateTime, Utc};
#[cfg(test)]
use mockall::automock;
use mycelium_base::utils::errors::MappedErrors;
use shaku::Interface;

/// Trace, quota and queue of the downstream service notifications
///
/// The three writes of a send are one port because they must be atomic: a
/// quota-accepted message must not fail to enqueue, and no message may be
/// enqueued without its trace row.
#[cfg_attr(test, automock)]
#[async_trait]
pub trait NotificationDispatchRegistration: Interface + Send + Sync {
    /// Register the dispatch rows and enqueue their messages, in one
    /// transaction, unless the batch would cross the sender hourly quota
    ///
    /// `rows` and `messages` are paired by position and share their ids. The
    /// batch is accepted or rejected as a whole.
    async fn register_and_enqueue(
        &self,
        sender_tag: &str,
        quota_per_hour: u32,
        rows: Vec<NewNotificationDispatch>,
        messages: Vec<MessageSendingEvent>,
    ) -> Result<QuotaOutcome, MappedErrors>;

    /// Delete the dispatch rows created before `cutoff`, returning how many
    async fn purge_older_than(
        &self,
        cutoff: DateTime<Utc>,
    ) -> Result<u64, MappedErrors>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn mock_answers_both_methods() -> Result<(), MappedErrors> {
        let mut repository = MockNotificationDispatchRegistration::new();

        repository
            .expect_register_and_enqueue()
            .withf(|sender_tag, quota_per_hour, rows, messages| {
                sender_tag == "service-01"
                    && *quota_per_hour == 10
                    && rows.is_empty()
                    && messages.is_empty()
            })
            .returning(|_, _, _, _| {
                Ok(QuotaOutcome::Exceeded {
                    retry_after_secs: 42,
                })
            });

        repository.expect_purge_older_than().returning(|_| Ok(3));

        let outcome = repository
            .register_and_enqueue("service-01", 10, vec![], vec![])
            .await?;

        assert_eq!(
            outcome,
            QuotaOutcome::Exceeded {
                retry_after_secs: 42
            }
        );
        assert_eq!(repository.purge_older_than(Utc::now()).await?, 3);

        Ok(())
    }
}
