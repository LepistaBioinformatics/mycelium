use crate::{
    models::{
        config::DbPoolProvider, webhook::WebHook as WebHookModel,
        webhook_execution::WebHookExecution as WebHookExecutionModel,
    },
    repositories::parse_optional_written_by,
    schema::{
        webhook as webhook_model, webhook_execution as webhook_execution_model,
    },
};

use async_trait::async_trait;
use chrono::{Duration, Local, NaiveDateTime};
use diesel::{
    expression::BoxableExpression,
    pg::Pg,
    prelude::*,
    sql_types::{Bool, Nullable},
};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{
            PayloadId, WebHook, WebHookExecutionStatus, WebHookPayloadArtifact,
            WebHookRetryPolicy, WebHookTrigger,
        },
    },
    entities::WebHookFetching,
};
use mycelium_base::{
    entities::{FetchManyResponseKind, FetchResponseKind},
    utils::errors::{fetching_err, MappedErrors},
};
use serde_json::from_value;
use shaku::Component;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookFetching)]
pub struct WebHookFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

#[async_trait]
impl WebHookFetching for WebHookFetchingSqlDbRepository {
    #[tracing::instrument(name = "get_webhook", skip_all)]
    async fn get(
        &self,
        id: Uuid,
    ) -> Result<FetchResponseKind<WebHook, Uuid>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let webhook = webhook_model::table
            .find(id)
            .select(WebHookModel::as_select())
            .first::<WebHookModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhook: {}", e))
            })?;

        match webhook {
            Some(record) => {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.method.map(|m| m.parse().unwrap()),
                    record.secret.map(|s| from_value(s).unwrap()),
                    parse_optional_written_by(record.created_by),
                );

                webhook.id = Some(record.id);
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());
                webhook.updated_by =
                    parse_optional_written_by(record.updated_by);

                webhook.redact_secret_token();

                Ok(FetchResponseKind::Found(webhook))
            }
            None => Ok(FetchResponseKind::NotFound(Some(id))),
        }
    }

    #[tracing::instrument(name = "list_webhooks", skip_all)]
    async fn list(
        &self,
        name: Option<String>,
        trigger: Option<WebHookTrigger>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let base_query = webhook_model::table;
        let mut count_query = base_query.into_boxed();
        let mut records_query = base_query.into_boxed();

        if let Some(name) = name {
            let dsl = webhook_model::name.ilike(format!("%{}%", name));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(trigger) = trigger {
            let dsl = webhook_model::trigger.eq(trigger.to_string());
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let records = records_query
            .select(WebHookModel::as_select())
            .order_by(webhook_model::created.desc())
            .limit(page_size)
            .offset(skip)
            .load::<WebHookModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhooks: {}", e))
            })?;

        if records.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let total = count_query
            .select(diesel::dsl::count_star())
            .first::<i64>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to count webhooks: {}", e))
            })?;

        let webhooks = records
            .into_iter()
            .map(|record| {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.method.map(|m| m.parse().unwrap()),
                    record.secret.map(|s| from_value(s).unwrap()),
                    parse_optional_written_by(record.created_by),
                );

                webhook.id = Some(record.id);
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());
                webhook.updated_by =
                    parse_optional_written_by(record.updated_by);

                webhook.redact_secret_token();
                webhook
            })
            .collect();

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total,
            skip: Some(skip),
            size: Some(page_size),
            records: webhooks,
        })
    }

    #[tracing::instrument(name = "list_webhooks_by_trigger", skip_all)]
    async fn list_by_trigger(
        &self,
        trigger: WebHookTrigger,
    ) -> Result<FetchManyResponseKind<WebHook>, MappedErrors> {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let webhooks = webhook_model::table
            .filter(webhook_model::trigger.eq(trigger.to_string()))
            .filter(webhook_model::is_active.eq(true))
            .select(WebHookModel::as_select())
            .load::<WebHookModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhooks: {}", e))
            })?;

        if webhooks.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let webhooks = webhooks
            .into_iter()
            .map(|record| {
                let mut webhook = WebHook::new(
                    record.name,
                    record.description,
                    record.url,
                    record.trigger.parse().unwrap(),
                    record.method.map(|m| m.parse().unwrap()),
                    record.secret.map(|s| from_value(s).unwrap()),
                    parse_optional_written_by(record.created_by),
                );

                webhook.id = Some(record.id);
                webhook.is_active = record.is_active;
                webhook.created =
                    record.created.and_local_timezone(Local).unwrap();
                webhook.updated = record
                    .updated
                    .map(|dt| dt.and_local_timezone(Local).unwrap());
                webhook.updated_by =
                    parse_optional_written_by(record.updated_by);

                webhook
            })
            .collect();

        Ok(FetchManyResponseKind::Found(webhooks))
    }

    #[tracing::instrument(name = "fetch_execution_event", skip_all)]
    async fn fetch_execution_event(
        &self,
        max_events: u32,
        max_attempts: u32,
        status: Option<Vec<WebHookExecutionStatus>>,
        retry_policy: WebHookRetryPolicy,
    ) -> Result<FetchManyResponseKind<WebHookPayloadArtifact>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {}", e))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let statuses = status
            .unwrap_or(vec![WebHookExecutionStatus::Pending])
            .into_iter()
            .map(|s| s.to_string())
            .collect::<Vec<String>>();

        // SAFETY INVARIANT (`retry_policy.visibility_timeout_in_secs`, from
        // `[core.webhook] visibilityTimeoutInSecs`): the whole batch is marked
        // `processing` here, up front, and then dispatched SEQUENTIALLY by the
        // caller. The window must therefore exceed the worst-case wall-clock of
        // the entire batch, not of one event -- otherwise a live-but-slow pod
        // has its still-undispatched rows reclaimed by another pod and the
        // event is double-sent, which is the exact defect this claim exists to
        // close. `consumeBatchSize * requestTimeoutInSecs` is the floor of that
        // bound; each event also pays a `list_by_trigger`, a KEK derivation, a
        // DEK fetch and a sequential secret-decryption loop. RAISING the batch
        // or the request timeout REQUIRES raising the window with it.
        let now = Local::now().naive_utc();

        let claimed = conn
            .transaction::<Vec<WebHookExecutionModel>, diesel::result::Error, _>(
                |conn| {
                    let rows = webhook_execution_model::table
                        .filter(
                            webhook_execution_model::attempts
                                .lt(max_attempts as i32),
                        )
                        .filter(due_for_dispatch(
                            &statuses,
                            max_attempts,
                            &retry_policy,
                            now,
                        ))
                        .order(webhook_execution_model::created.desc())
                        .limit(max_events as i64)
                        .select(WebHookExecutionModel::as_select())
                        .for_update()
                        .skip_locked()
                        .load::<WebHookExecutionModel>(conn)?;

                    let ids =
                        rows.iter().map(|row| row.id).collect::<Vec<_>>();

                    if ids.is_empty() {
                        return Ok(vec![]);
                    }

                    diesel::update(
                        webhook_execution_model::table
                            .filter(webhook_execution_model::id.eq_any(&ids)),
                    )
                    .set((
                        webhook_execution_model::status
                            .eq(WebHookExecutionStatus::Processing.to_string()),
                        webhook_execution_model::claimed_at.eq(now),
                    ))
                    .execute(conn)?;

                    Ok(rows)
                },
            )
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to claim webhook execution events: {e}"
                ))
            })?;

        let execution_events = claimed;

        let execution_events = execution_events
            .into_iter()
            .map(|record| WebHookPayloadArtifact {
                id: Some(record.id),
                payload: record.payload.to_string(),
                payload_id: PayloadId::from_str(&record.payload_id).unwrap(),
                trigger: record.trigger.parse().unwrap(),
                // An attempt that failed before any hook was contacted has
                // nothing to propagate, and the column then holds a JSONB
                // `null` -- which is not a sequence. Degrade to `None` rather
                // than unwrap: this runs inside the dispatcher task, where a
                // panic takes the whole queue down over one malformed row.
                propagations: record
                    .propagations
                    .and_then(|propagations| from_value(propagations).ok()),
                encrypted: record.encrypted,
                attempts: Some(record.attempts as u8),
                attempted: record
                    .attempted
                    .map(|a| a.and_local_timezone(Local).unwrap()),
                created: Some(
                    record.created.and_local_timezone(Local).unwrap(),
                ),
                status: record
                    .status
                    .map(|s| WebHookExecutionStatus::from_str(&s).unwrap()),
            })
            .collect();

        Ok(FetchManyResponseKind::Found(execution_events))
    }
}

/// The selector for events that are actually owed an attempt right now
///
/// Two branches, OR-ed:
///
/// - an event in one of the requested statuses whose back-off has elapsed, and
/// - an event some pod claimed and never finished, past the visibility window.
///
/// The second branch is always included, whatever the caller asked for: a
/// claim left behind by a pod that died has to be recoverable regardless of the
/// status filter in force.
///
/// The exponent lives in Rust rather than in SQL. `min(base * 2^n, cap)` would
/// need `power()` and interval arithmetic to be expressed in the query, which
/// is Postgres-only and would fork this adapter from its SQLite twin. Expanding
/// it into one OR-term per attempt tier keeps the whole predicate inside the
/// diesel DSL, keeps every value bound, and costs `maxAttempts` terms -- five
/// at the default.
///
type ClaimPredicate = Box<
    dyn BoxableExpression<
        webhook_execution_model::table,
        Pg,
        SqlType = Nullable<Bool>,
    >,
>;

fn due_for_dispatch(
    statuses: &[String],
    max_attempts: u32,
    retry_policy: &WebHookRetryPolicy,
    now: NaiveDateTime,
) -> ClaimPredicate {
    let mut predicate: ClaimPredicate = Box::new(
        webhook_execution_model::status
            .eq_any(statuses.to_owned())
            .and(webhook_execution_model::attempted.is_null()),
    );

    // One OR-term per tier, so the chain is bounded twice over: `attempts` is a
    // `u8` on the domain artifact and can never exceed 255, and an unclamped
    // `maxAttempts` would otherwise emit one term per configured attempt --
    // a 10 000-term predicate for a 10 000 ceiling.
    let tiers = max_attempts.min(u8::MAX as u32 + 1);

    for attempt in 0..tiers {
        let cutoff = now
            - Duration::seconds(retry_policy.backoff_in_secs(attempt) as i64);

        predicate = Box::new(
            predicate.or(webhook_execution_model::status
                .eq_any(statuses.to_owned())
                .and(webhook_execution_model::attempts.eq(attempt as i32))
                .and(webhook_execution_model::attempted.lt(cutoff))),
        );
    }

    let stale_cutoff =
        now - Duration::seconds(retry_policy.visibility_timeout_in_secs);

    Box::new(
        predicate.or(webhook_execution_model::status
            .eq(WebHookExecutionStatus::Processing.to_string())
            .and(webhook_execution_model::claimed_at.lt(stale_cutoff))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::DbPool;
    use diesel::{
        r2d2::{ConnectionManager, Pool},
        sql_query, PgConnection,
    };
    use lazy_static::lazy_static;
    use myc_core::domain::dtos::webhook::WebHookTrigger;
    use std::sync::{Barrier, Mutex};

    /// Postgres URL for the live-database tests below
    ///
    /// These prove the one requirement no unit test can reach -- that two
    /// replicas never claim the same event -- which needs a real `FOR UPDATE
    /// SKIP LOCKED`. Without the variable they report the skip and pass, so
    /// `cargo test --workspace` stays green on a machine with no Postgres.
    const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

    lazy_static! {
        /// The tests share one table, so they may not interleave.
        static ref SERIALISE: Mutex<()> = Mutex::new(());
    }

    fn pool() -> Option<DbPool> {
        let url = std::env::var(DATABASE_URL_VAR).ok()?;

        Some(
            Pool::builder()
                .max_size(8)
                .build(ConnectionManager::<PgConnection>::new(url))
                .expect("failed to build the test pool"),
        )
    }

    struct TestPool(DbPool);

    impl DbPoolProvider for TestPool {
        fn get_pool(&self) -> DbPool {
            self.0.clone()
        }
    }

    fn repository(pool: &DbPool) -> WebHookFetchingSqlDbRepository {
        WebHookFetchingSqlDbRepository {
            db_config: Arc::new(TestPool(pool.to_owned())),
        }
    }

    fn reset_schema(pool: &DbPool) {
        let conn = &mut pool.get().expect("failed to take a test connection");

        for statement in [
            "DROP TABLE IF EXISTS webhook_execution",
            "CREATE TABLE webhook_execution (
                id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                trigger VARCHAR(255) NOT NULL,
                payload TEXT NOT NULL,
                payload_id VARCHAR(255) NOT NULL,
                encrypted BOOLEAN DEFAULT FALSE,
                attempts INT DEFAULT 0,
                created TIMESTAMPTZ DEFAULT now(),
                attempted TIMESTAMPTZ DEFAULT NULL,
                claimed_at TIMESTAMPTZ DEFAULT NULL,
                status VARCHAR(100) DEFAULT NULL,
                propagations JSONB
            )",
            "CREATE INDEX idx_webhook_execution_claim
                ON webhook_execution (status, attempts, attempted)",
        ] {
            sql_query(statement)
                .execute(conn)
                .expect("failed to prepare the test schema");
        }
    }

    fn seed(
        pool: &DbPool,
        events: usize,
        status: WebHookExecutionStatus,
        attempts: i32,
        attempted: Option<NaiveDateTime>,
        claimed_at: Option<NaiveDateTime>,
    ) -> Vec<Uuid> {
        let conn = &mut pool.get().expect("failed to take a test connection");

        (0..events)
            .map(|index| {
                let row = WebHookExecutionModel {
                    id: Uuid::new_v4(),
                    trigger: WebHookTrigger::SubscriptionAccountCreated
                        .to_string(),
                    payload: format!("{{\"event\":{index}}}"),
                    payload_id: Uuid::new_v4().to_string(),
                    created: Local::now().naive_utc(),
                    status: Some(status.to_string()),
                    attempts,
                    attempted,
                    claimed_at,
                    propagations: None,
                    encrypted: None,
                };

                diesel::insert_into(webhook_execution_model::table)
                    .values(&row)
                    .returning(webhook_execution_model::id)
                    .get_result::<Uuid>(conn)
                    .expect("failed to seed a webhook execution event")
            })
            .collect()
    }

    fn claim(
        repository: &WebHookFetchingSqlDbRepository,
        retry_policy: WebHookRetryPolicy,
    ) -> Vec<Uuid> {
        let claimed =
            futures::executor::block_on(repository.fetch_execution_event(
                100,
                5,
                Some(vec![
                    WebHookExecutionStatus::Pending,
                    WebHookExecutionStatus::Failed,
                ]),
                retry_policy,
            ))
            .expect("the claim itself failed");

        match claimed {
            FetchManyResponseKind::Found(events) => {
                events.into_iter().filter_map(|event| event.id).collect()
            }
            other => panic!("unexpected claim response: {other:?}"),
        }
    }

    fn policy() -> WebHookRetryPolicy {
        WebHookRetryPolicy::new(30, 3600, 900)
    }

    fn skip_without_database() -> Option<DbPool> {
        let Some(pool) = pool() else {
            eprintln!(
                "skipping: set {DATABASE_URL_VAR} to run the live claim tests"
            );

            return None;
        };

        Some(pool)
    }

    #[test]
    fn a_claimed_batch_is_invisible_to_the_next_claim() {
        let _guard = SERIALISE.lock().unwrap();
        let Some(pool) = skip_without_database() else {
            return;
        };

        reset_schema(&pool);
        let seeded =
            seed(&pool, 5, WebHookExecutionStatus::Pending, 0, None, None);

        let repository = repository(&pool);

        let first = claim(&repository, policy());
        assert_eq!(first.len(), seeded.len());

        // The whole point: the same events are gone for everyone else until
        // either the dispatch resolves them or the lease expires.
        let second = claim(&repository, policy());
        assert!(
            second.is_empty(),
            "a claimed batch was handed out a second time: {second:?}"
        );
    }

    #[test]
    fn two_simultaneous_claims_never_overlap() {
        let _guard = SERIALISE.lock().unwrap();
        let Some(pool) = skip_without_database() else {
            return;
        };

        reset_schema(&pool);
        let seeded =
            seed(&pool, 40, WebHookExecutionStatus::Pending, 0, None, None);

        let barrier = Arc::new(Barrier::new(2));

        let claims = (0..2)
            .map(|_| {
                let pool = pool.to_owned();
                let barrier = barrier.to_owned();

                std::thread::spawn(move || {
                    let repository = repository(&pool);
                    barrier.wait();
                    claim(&repository, policy())
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|handle| handle.join().expect("a claiming thread panicked"))
            .collect::<Vec<_>>();

        let overlap = claims[0]
            .iter()
            .filter(|id| claims[1].contains(id))
            .collect::<Vec<_>>();

        assert!(
            overlap.is_empty(),
            "both replicas claimed the same events: {overlap:?}"
        );

        // Observed split is 40/0, not 20/20, and that is `SKIP LOCKED` working
        // rather than a flaw in the test: whichever transaction gets there
        // first locks every candidate row, and the other skips all of them
        // instead of waiting. Either way each event is claimed exactly once,
        // which is the property under test. Without the claim both threads
        // would return all 40 and this sum would be 80.
        assert_eq!(
            claims[0].len() + claims[1].len(),
            seeded.len(),
            "events were claimed twice, or lost"
        );
    }

    #[test]
    fn a_dead_pods_claim_is_reclaimed_only_after_the_window() {
        let _guard = SERIALISE.lock().unwrap();
        let Some(pool) = skip_without_database() else {
            return;
        };

        let now = Local::now().naive_utc();

        reset_schema(&pool);
        seed(
            &pool,
            3,
            WebHookExecutionStatus::Processing,
            0,
            None,
            Some(now - Duration::seconds(60)),
        );

        let repository = repository(&pool);

        let inside_the_window = claim(&repository, policy());
        assert!(
            inside_the_window.is_empty(),
            "an event a live pod is still working on was stolen: {inside_the_window:?}"
        );

        reset_schema(&pool);
        let abandoned = seed(
            &pool,
            3,
            WebHookExecutionStatus::Processing,
            0,
            None,
            Some(now - Duration::seconds(1_000)),
        );

        let reclaimed = claim(&repository, policy());
        assert_eq!(
            reclaimed.len(),
            abandoned.len(),
            "an event left behind by a dead pod was never reclaimed"
        );
    }

    #[test]
    fn a_recent_failure_serves_its_backoff_before_being_claimed_again() {
        let _guard = SERIALISE.lock().unwrap();
        let Some(pool) = skip_without_database() else {
            return;
        };

        let now = Local::now().naive_utc();

        reset_schema(&pool);

        // One attempt spent, so the next one is owed `backoff(1)` = 60s.
        seed(
            &pool,
            3,
            WebHookExecutionStatus::Failed,
            1,
            Some(now - Duration::seconds(45)),
            None,
        );

        let repository = repository(&pool);

        let too_soon = claim(&repository, policy());
        assert!(
            too_soon.is_empty(),
            "a failure 45s old was retried inside its 60s back-off: {too_soon:?}"
        );

        reset_schema(&pool);
        let due = seed(
            &pool,
            3,
            WebHookExecutionStatus::Failed,
            1,
            Some(now - Duration::seconds(75)),
            None,
        );

        let claimed = claim(&repository, policy());
        assert_eq!(
            claimed.len(),
            due.len(),
            "a failure past its back-off was never retried"
        );
    }
}
