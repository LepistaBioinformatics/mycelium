use crate::models::config::DbPoolProvider;

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDateTime};
use diesel::{
    pg::Pg,
    prelude::*,
    query_builder::{BoxedSqlQuery, SqlQuery},
    sql_types::{BigInt, Integer, Nullable, Text, Timestamptz},
};
use myc_core::domain::{
    dtos::{
        native_error_codes::NativeErrorCodes,
        webhook::{
            WebHookDispatchState, WebHookExecutionEvent,
            WebHookExecutionStatus, WebHookExecutionSummary,
            WebHookPropagationSummary, WebHookTrigger,
        },
    },
    entities::WebHookExecutionFetching,
};
use mycelium_base::{
    entities::FetchManyResponseKind,
    utils::errors::{fetching_err, MappedErrors},
};
use shaku::Component;
use std::{str::FromStr, sync::Arc};
use uuid::Uuid;

#[derive(Component)]
#[shaku(interface = WebHookExecutionFetching)]
pub struct WebHookExecutionFetchingSqlDbRepository {
    #[shaku(inject)]
    pub db_config: Arc<dyn DbPoolProvider>,
}

/// One row of the `webhook_execution_event` view.
#[derive(QueryableByName)]
struct EventRow {
    #[diesel(sql_type = diesel::sql_types::Uuid)]
    id: Uuid,
    #[diesel(sql_type = Text)]
    trigger: String,
    #[diesel(sql_type = Text)]
    payload_id: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Integer)]
    attempts: i32,
    #[diesel(sql_type = Timestamptz)]
    created: NaiveDateTime,
    #[diesel(sql_type = Nullable<Timestamptz>)]
    attempted: Option<NaiveDateTime>,
    #[diesel(sql_type = BigInt)]
    propagations_total: i64,
    #[diesel(sql_type = BigInt)]
    propagations_failed: i64,
    #[diesel(sql_type = Nullable<Integer>)]
    last_response_status: Option<i32>,
}

#[derive(QueryableByName)]
struct SummaryRow {
    #[diesel(sql_type = Text)]
    trigger: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = BigInt)]
    count: i64,
    #[diesel(sql_type = Timestamptz)]
    latest: NaiveDateTime,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

/// SQL filter for the optional trigger/status (values are always bound),
/// plus the number of placeholders it uses.
fn where_clause(
    trigger: &Option<WebHookTrigger>,
    status: &Option<WebHookExecutionStatus>,
) -> (String, usize) {
    let mut conditions = vec![];
    if trigger.is_some() {
        conditions.push(format!("trigger = ${}", conditions.len() + 1));
    }
    if status.is_some() {
        conditions.push(format!("status = ${}", conditions.len() + 1));
    }
    match conditions.is_empty() {
        true => (String::new(), 0),
        false => (
            format!(" WHERE {}", conditions.join(" AND ")),
            conditions.len(),
        ),
    }
}

fn bind_filters<'a>(
    mut query: BoxedSqlQuery<'a, Pg, SqlQuery>,
    trigger: &Option<WebHookTrigger>,
    status: &Option<WebHookExecutionStatus>,
) -> BoxedSqlQuery<'a, Pg, SqlQuery> {
    if let Some(trigger) = trigger {
        query = query.bind::<Text, _>(trigger.to_string());
    }
    if let Some(status) = status {
        query = query.bind::<Text, _>(status.to_string());
    }
    query
}

fn local(value: NaiveDateTime) -> DateTime<Local> {
    value.and_utc().with_timezone(&Local)
}

fn to_event(row: EventRow) -> Result<WebHookExecutionEvent, MappedErrors> {
    Ok(WebHookExecutionEvent {
        id: row.id,
        trigger: WebHookTrigger::from_str(&row.trigger).map_err(|e| {
            fetching_err(format!("Invalid webhook trigger: {e}"))
        })?,
        payload_id: row.payload_id,
        dispatch: WebHookDispatchState {
            status: WebHookExecutionStatus::from_str(&row.status)?,
            attempts: row.attempts.max(0) as u32,
            created: local(row.created),
            attempted: row.attempted.map(local),
        },
        propagations: WebHookPropagationSummary {
            total: row.propagations_total as u32,
            failed: row.propagations_failed as u32,
            last_response_status: row
                .last_response_status
                .and_then(|status| u16::try_from(status).ok()),
        },
    })
}

fn to_summary(
    row: SummaryRow,
) -> Result<WebHookExecutionSummary, MappedErrors> {
    Ok(WebHookExecutionSummary {
        trigger: WebHookTrigger::from_str(&row.trigger).map_err(|e| {
            fetching_err(format!("Invalid webhook trigger: {e}"))
        })?,
        status: WebHookExecutionStatus::from_str(&row.status)?,
        count: row.count as u64,
        latest: local(row.latest),
    })
}

#[async_trait]
impl WebHookExecutionFetching for WebHookExecutionFetchingSqlDbRepository {
    #[tracing::instrument(name = "list_webhook_execution_events", skip_all)]
    async fn list_events(
        &self,
        trigger: Option<WebHookTrigger>,
        status: Option<WebHookExecutionStatus>,
        page_size: Option<i32>,
        skip: Option<i32>,
    ) -> Result<FetchManyResponseKind<WebHookExecutionEvent>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let page_size = page_size.unwrap_or(20).clamp(1, 100) as i64;
        let skip = skip.unwrap_or(0).max(0) as i64;
        let (filter, used) = where_clause(&trigger, &status);

        let rows = bind_filters(
            diesel::sql_query(format!(
                "SELECT * FROM webhook_execution_event{filter} \
                 ORDER BY created DESC LIMIT ${} OFFSET ${}",
                used + 1,
                used + 2
            ))
            .into_boxed(),
            &trigger,
            &status,
        )
        .bind::<BigInt, _>(page_size)
        .bind::<BigInt, _>(skip)
        .load::<EventRow>(conn)
        .map_err(|e| {
            fetching_err(format!("Failed to list webhook executions: {e}"))
        })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        let total = bind_filters(
            diesel::sql_query(format!(
                "SELECT count(*) AS count FROM webhook_execution_event{filter}"
            ))
            .into_boxed(),
            &trigger,
            &status,
        )
        .get_result::<CountRow>(conn)
        .map_err(|e| {
            fetching_err(format!("Failed to count webhook executions: {e}"))
        })?;

        Ok(FetchManyResponseKind::FoundPaginated {
            count: total.count,
            skip: Some(skip),
            size: Some(page_size),
            records: rows
                .into_iter()
                .map(to_event)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    #[tracing::instrument(
        name = "summarize_webhook_execution_events",
        skip_all
    )]
    async fn summarize_events(
        &self,
        since: Option<DateTime<Local>>,
    ) -> Result<FetchManyResponseKind<WebHookExecutionSummary>, MappedErrors>
    {
        let conn = &mut self.db_config.get_pool().get().map_err(|e| {
            fetching_err(format!("Failed to get DB connection: {e}"))
                .with_code(NativeErrorCodes::MYC00001)
        })?;

        let mut query = diesel::sql_query(format!(
            "SELECT trigger, status, count(*) AS count, max(created) AS latest \
             FROM webhook_execution_event{} \
             GROUP BY trigger, status ORDER BY trigger, status",
            if since.is_some() { " WHERE created >= $1" } else { "" }
        ))
        .into_boxed::<Pg>();

        if let Some(since) = since {
            query = query.bind::<Timestamptz, _>(since.naive_utc());
        }

        let rows = query.load::<SummaryRow>(conn).map_err(|e| {
            fetching_err(format!("Failed to summarize webhook executions: {e}"))
        })?;

        if rows.is_empty() {
            return Ok(FetchManyResponseKind::NotFound);
        }

        Ok(FetchManyResponseKind::Found(
            rows.into_iter()
                .map(to_summary)
                .collect::<Result<Vec<_>, _>>()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::config::DbPool;
    use diesel::{
        r2d2::{ConnectionManager, Pool},
        sql_query, PgConnection,
    };

    /// Same variable as the live webhook claim tests. Without it the live
    /// test below reports the skip and passes.
    const DATABASE_URL_VAR: &str = "MYC_TEST_DATABASE_URL";

    /// Own schema, so this test never races the claim tests, which recreate
    /// `webhook_execution` in the default schema.
    const SCHEMA: &str = "myc_webhook_execution_event_test";

    /// The view exactly as the migration creates it (the GRANT needs psql
    /// variables and is irrelevant here).
    const VIEW_MIGRATION: &str = include_str!(
        "../../../sql/migrations/20261022_01_webhook_execution_event_view.sql"
    );

    struct TestPool(DbPool);

    impl DbPoolProvider for TestPool {
        fn get_pool(&self) -> DbPool {
            self.0.clone()
        }
    }

    fn pool() -> Option<DbPool> {
        let url = std::env::var(DATABASE_URL_VAR).ok()?;
        let separator = if url.contains('?') { '&' } else { '?' };
        let url = format!("{url}{separator}options=-csearch_path%3D{SCHEMA}");

        Some(
            Pool::builder()
                .max_size(2)
                .build(ConnectionManager::<PgConnection>::new(url))
                .expect("failed to build the test pool"),
        )
    }

    fn prepare(pool: &DbPool) {
        let conn = &mut pool.get().expect("failed to take a test connection");
        let view = VIEW_MIGRATION
            .split("GRANT SELECT")
            .next()
            .expect("migration has a view");

        for statement in [
            format!("CREATE SCHEMA IF NOT EXISTS {SCHEMA}"),
            "DROP VIEW IF EXISTS webhook_execution_event".to_string(),
            "DROP TABLE IF EXISTS webhook_execution".to_string(),
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
            )"
            .to_string(),
            view.to_string(),
            "INSERT INTO webhook_execution
                (trigger, payload, payload_id, attempts, status, attempted,
                 propagations)
             VALUES
                ('userAccount.created', '{\"secret\":\"do-not-leak\"}', 'p1',
                 2, 'failed', now(),
                 '[{\"url\":\"https://hooks.example.com/private\",\"status\":200,\"body\":\"sensitive\",\"datetime\":\"2026-10-22T10:00:00Z\"},
                   {\"url\":\"https://hooks.example.com/private\",\"status\":503,\"body\":\"sensitive\",\"datetime\":\"2026-10-22T10:01:00Z\"}]'),
                ('userAccount.created', 'x', 'p2', 0, NULL, NULL, NULL),
                ('subscriptionAccount.created', 'x', 'p3', 1, 'success', now(),
                 'null')"
                .to_string(),
        ] {
            sql_query(statement)
                .execute(conn)
                .expect("failed to prepare the test schema");
        }
    }

    #[tokio::test]
    async fn live_view_lists_and_summarizes_events() -> Result<(), MappedErrors>
    {
        let Some(pool) = pool() else {
            eprintln!(
                "skipping: set {DATABASE_URL_VAR} to run the live execution \
                 event view test"
            );
            return Ok(());
        };
        prepare(&pool);

        let repo = WebHookExecutionFetchingSqlDbRepository {
            db_config: Arc::new(TestPool(pool)),
        };

        let FetchManyResponseKind::FoundPaginated { count, records, .. } = repo
            .list_events(
                Some(WebHookTrigger::UserAccountCreated),
                None,
                None,
                None,
            )
            .await?
        else {
            panic!("expected paginated events");
        };
        assert_eq!(count, 2);

        let failed = records
            .iter()
            .find(|e| e.payload_id == "p1")
            .expect("p1 listed");
        assert_eq!(failed.dispatch.status, WebHookExecutionStatus::Failed);
        assert_eq!(failed.dispatch.attempts, 2);
        assert_eq!(failed.propagations.total, 2);
        assert_eq!(failed.propagations.failed, 1);
        assert_eq!(failed.propagations.last_response_status, Some(503));

        // NULL status and NULL propagations
        let pending = records
            .iter()
            .find(|e| e.payload_id == "p2")
            .expect("p2 listed");
        assert_eq!(pending.dispatch.status, WebHookExecutionStatus::Unknown);
        assert_eq!(pending.propagations.total, 0);

        let serialized = serde_json::to_string(&records).unwrap();
        assert!(!serialized.contains("do-not-leak"));
        assert!(!serialized.contains("hooks.example.com"));
        assert!(!serialized.contains("sensitive"));

        // JSON `null` propagations and a status filter
        let FetchManyResponseKind::FoundPaginated { records, .. } = repo
            .list_events(
                None,
                Some(WebHookExecutionStatus::Success),
                None,
                None,
            )
            .await?
        else {
            panic!("expected the successful event");
        };
        assert_eq!(records.len(), 1);
        assert_eq!(records[0].propagations.total, 0);

        let FetchManyResponseKind::Found(summary) =
            repo.summarize_events(None).await?
        else {
            panic!("expected a summary");
        };
        assert_eq!(summary.len(), 3);
        assert!(matches!(
            repo.summarize_events(Some(
                Local::now() + chrono::Duration::days(1)
            ))
            .await?,
            FetchManyResponseKind::NotFound
        ));

        Ok(())
    }

    #[test]
    fn filters_use_numbered_placeholders() {
        assert_eq!(where_clause(&None, &None), (String::new(), 0));
        assert_eq!(
            where_clause(
                &Some(WebHookTrigger::UserAccountCreated),
                &Some(WebHookExecutionStatus::Failed)
            ),
            (" WHERE trigger = $1 AND status = $2".to_string(), 2)
        );
        assert_eq!(
            where_clause(&None, &Some(WebHookExecutionStatus::Failed)),
            (" WHERE status = $1".to_string(), 1)
        );
    }
}
