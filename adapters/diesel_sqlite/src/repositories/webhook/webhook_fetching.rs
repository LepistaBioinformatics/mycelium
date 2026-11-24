use super::map_model_to_dto;
use crate::{
    config::SqliteDbPoolProvider,
    models::{
        webhook::WebHook as WebHookModel,
        webhook_execution::WebHookExecution as WebHookExecutionModel,
    },
    schema::{webhook, webhook_execution},
    types::{
        json_from_text, naive_timestamp_from_text, naive_timestamp_to_text,
        uuid_to_text,
    },
};

use async_trait::async_trait;
use chrono::{Duration, Local, NaiveDateTime};
use diesel::{
    expression::BoxableExpression,
    prelude::*,
    sql_types::{Bool, Nullable},
    sqlite::Sqlite,
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
use shaku::Component;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookFetching)]
pub struct WebHookFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
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

        let webhook_row = webhook::table
            .find(uuid_to_text(&id))
            .select(WebHookModel::as_select())
            .first::<WebHookModel>(conn)
            .optional()
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhook: {}", e))
            })?;

        match webhook_row {
            Some(record) => {
                Ok(FetchResponseKind::Found(map_model_to_dto(record, true)))
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

        let base_query = webhook::table;
        let mut count_query = base_query.into_boxed();
        let mut records_query = base_query.into_boxed();

        if let Some(name) = name {
            // SQLite's LIKE is case-insensitive for ASCII by default,
            // matching postgres's ILIKE for the common case.
            let dsl = webhook::name.like(format!("%{}%", name));
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        if let Some(trigger) = trigger {
            let dsl = webhook::trigger.eq(trigger.to_string());
            records_query = records_query.filter(dsl.clone());
            count_query = count_query.filter(dsl);
        }

        let page_size = page_size.unwrap_or(10) as i64;
        let skip = skip.unwrap_or(0) as i64;

        let records = records_query
            .select(WebHookModel::as_select())
            .order_by(webhook::created.desc())
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
            .map(|record| map_model_to_dto(record, true))
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

        let webhooks = webhook::table
            .filter(webhook::trigger.eq(trigger.to_string()))
            .filter(webhook::is_active.eq(true))
            .select(WebHookModel::as_select())
            .load::<WebHookModel>(conn)
            .map_err(|e| {
                fetching_err(format!("Failed to fetch webhooks: {}", e))
            })?;

        if webhooks.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        // Unlike `get`/`list`, the secret is NOT redacted here: this method
        // feeds the internal dispatcher that signs outgoing payloads, which
        // needs the real secret (mirrors the postgres repo exactly).
        let webhooks = webhooks
            .into_iter()
            .map(|record| map_model_to_dto(record, false))
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

        // No claim here, and none needed: standalone is a single process with a
        // single dispatcher, so there is no second consumer to race against and
        // nothing to lock against it. `SKIP LOCKED` has no SQLite equivalent
        // anyway. The Postgres twin's `claimed_at`/`processing` lease is absent
        // from this schema for the same reason -- the same split the email
        // queue's two adapters already carry.
        let now = Local::now().naive_utc();

        let execution_events = webhook_execution::table
            .filter(webhook_execution::attempts.lt(max_attempts as i32))
            .filter(due_for_dispatch(
                &statuses,
                max_attempts,
                &retry_policy,
                now,
            ))
            .order(webhook_execution::created.desc())
            .limit(max_events as i64)
            .select(WebHookExecutionModel::as_select())
            .load::<WebHookExecutionModel>(conn)
            .map_err(|e| {
                fetching_err(format!(
                    "Failed to fetch webhook execution events: {e}"
                ))
            })?;

        let execution_events = execution_events
            .into_iter()
            .map(|record| WebHookPayloadArtifact {
                id: Some(crate::types::uuid_from_text(&record.id).unwrap()),
                payload: record.payload.to_string(),
                payload_id: PayloadId::from_str(&record.payload_id).unwrap(),
                trigger: record.trigger.parse().unwrap(),
                // An attempt that failed before any hook was contacted has
                // nothing to propagate, and the column then holds a JSON
                // `null` -- which is not a sequence. Degrade to `None` rather
                // than unwrap: this runs inside the dispatcher task, where a
                // panic takes the whole queue down over one malformed row.
                propagations: record.propagations.as_deref().and_then(|p| {
                    json_from_text(p)
                        .ok()
                        .and_then(|value| serde_json::from_value(value).ok())
                }),
                encrypted: record.encrypted,
                attempts: Some(record.attempts as u8),
                attempted: record.attempted.map(|a| {
                    naive_timestamp_from_text(&a)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap()
                }),
                created: Some(
                    naive_timestamp_from_text(&record.created)
                        .unwrap()
                        .and_local_timezone(Local)
                        .unwrap(),
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
/// The back-off half of the Postgres twin's predicate, and only that half: one
/// OR-term per attempt tier, comparing the row's `attempted` against its own
/// precomputed cutoff. The exponent is computed in Rust so the expression stays
/// inside the diesel DSL and stays identical across both backends.
///
/// `attempted` is TEXT here. `naive_timestamp_to_text` writes a fixed-width
/// date-and-time prefix with an optional fractional tail, so lexicographic
/// order is chronological order and `<` means what it looks like. The cutoff is
/// formatted once and bound -- never concatenated into the query.
///
type ClaimPredicate = Box<
    dyn BoxableExpression<
        webhook_execution::table,
        Sqlite,
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
        webhook_execution::status
            .eq_any(statuses.to_owned())
            .and(webhook_execution::attempted.is_null()),
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
            predicate.or(webhook_execution::status
                .eq_any(statuses.to_owned())
                .and(webhook_execution::attempts.eq(attempt as i32))
                .and(
                    webhook_execution::attempted
                        .lt(naive_timestamp_to_text(&cutoff)),
                )),
        );
    }

    predicate
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        test_support::setup_temp_db,
        types::{naive_timestamp_to_text, uuid_to_text},
    };
    use myc_core::domain::dtos::webhook::WebHookTrigger;

    fn seed(
        provider: &Arc<dyn SqliteDbPoolProvider>,
        events: usize,
        status: WebHookExecutionStatus,
        attempts: i32,
        attempted: Option<NaiveDateTime>,
    ) -> usize {
        let conn = &mut provider.get_pool().get().unwrap();

        for index in 0..events {
            let row = WebHookExecutionModel {
                id: uuid_to_text(&Uuid::new_v4()),
                trigger: WebHookTrigger::SubscriptionAccountCreated.to_string(),
                payload: format!("{{\"event\":{index}}}"),
                payload_id: Uuid::new_v4().to_string(),
                created: naive_timestamp_to_text(&Local::now().naive_utc()),
                status: Some(status.to_string()),
                attempts,
                attempted: attempted.as_ref().map(naive_timestamp_to_text),
                propagations: None,
                encrypted: None,
            };

            diesel::insert_into(webhook_execution::table)
                .values(&row)
                .execute(conn)
                .unwrap();
        }

        events
    }

    async fn claim(
        repository: &WebHookFetchingSqlDbRepository,
        retry_policy: WebHookRetryPolicy,
    ) -> usize {
        let claimed = repository
            .fetch_execution_event(
                100,
                5,
                Some(vec![
                    WebHookExecutionStatus::Pending,
                    WebHookExecutionStatus::Failed,
                ]),
                retry_policy,
            )
            .await
            .unwrap();

        match claimed {
            FetchManyResponseKind::Found(events) => events.len(),
            other => panic!("unexpected claim response: {other:?}"),
        }
    }

    /// The back-off boundary, on the tier where it actually bites
    ///
    /// `attempts = 1` owes `backoff(1)` = 60s. Seeding `attempted` at 45s and
    /// then 75s straddles that, which is the only shape that proves anything
    /// here: a policy with a zero base and a zero cap would pass even if
    /// `backoff_in_secs` were ignored outright, since every tier's cutoff would
    /// collapse onto `now`.
    ///
    /// It is also the only place the TEXT storage of `attempted` is compared
    /// against a non-trivial boundary. `naive_timestamp_to_text` writes a
    /// fixed-width date and time, so lexicographic order is chronological
    /// order -- if that ever stopped holding, this is the test that would say
    /// so.
    ///
    #[tokio::test]
    async fn the_backoff_boundary_holds_against_text_timestamps() {
        let now = Local::now().naive_utc();

        let too_soon_db = setup_temp_db();
        let seeded = seed(
            &too_soon_db.provider,
            3,
            WebHookExecutionStatus::Failed,
            1,
            Some(now - Duration::seconds(45)),
        );

        let repository = WebHookFetchingSqlDbRepository {
            db_config: too_soon_db.provider.to_owned(),
        };

        assert_eq!(
            claim(&repository, WebHookRetryPolicy::new(30, 3600, 900)).await,
            0,
            "a failure 45s old was retried inside its 60s back-off"
        );

        let due_db = setup_temp_db();
        seed(
            &due_db.provider,
            seeded,
            WebHookExecutionStatus::Failed,
            1,
            Some(now - Duration::seconds(75)),
        );

        let repository = WebHookFetchingSqlDbRepository {
            db_config: due_db.provider.to_owned(),
        };

        assert_eq!(
            claim(&repository, WebHookRetryPolicy::new(30, 3600, 900)).await,
            seeded,
            "a failure past its 60s back-off was never retried"
        );
    }

    /// A never-attempted event is due at once, whatever the policy says
    #[tokio::test]
    async fn a_never_attempted_event_is_not_held_by_the_backoff() {
        let db = setup_temp_db();
        let seeded =
            seed(&db.provider, 2, WebHookExecutionStatus::Pending, 0, None);

        let repository = WebHookFetchingSqlDbRepository {
            db_config: db.provider.to_owned(),
        };

        assert_eq!(
            claim(&repository, WebHookRetryPolicy::new(3600, 3600, 900)).await,
            seeded,
            "a brand-new event was held back by a back-off it never earned"
        );
    }
}
