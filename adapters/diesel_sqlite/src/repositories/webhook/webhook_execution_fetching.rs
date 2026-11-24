use crate::{config::SqliteDbPoolProvider, types::naive_timestamp_from_text};

use async_trait::async_trait;
use chrono::{DateTime, Local, NaiveDateTime};
use diesel::{
    prelude::*,
    sql_types::{BigInt, Integer, Nullable, Text},
    sqlite::Sqlite,
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
    pub db_config: Arc<dyn SqliteDbPoolProvider>,
}

/// One row of the `webhook_execution_event` view.
#[derive(QueryableByName)]
struct EventRow {
    #[diesel(sql_type = Text)]
    id: String,
    #[diesel(sql_type = Text)]
    trigger: String,
    #[diesel(sql_type = Text)]
    payload_id: String,
    #[diesel(sql_type = Text)]
    status: String,
    #[diesel(sql_type = Integer)]
    attempts: i32,
    #[diesel(sql_type = Text)]
    created: String,
    #[diesel(sql_type = Nullable<Text>)]
    attempted: Option<String>,
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
    #[diesel(sql_type = Text)]
    latest: String,
}

#[derive(QueryableByName)]
struct CountRow {
    #[diesel(sql_type = BigInt)]
    count: i64,
}

/// SQL filter for the optional trigger/status; values are always bound.
fn where_clause(
    trigger: &Option<WebHookTrigger>,
    status: &Option<WebHookExecutionStatus>,
) -> String {
    let mut conditions = vec![];
    if trigger.is_some() {
        conditions.push("trigger = ?");
    }
    if status.is_some() {
        conditions.push("status = ?");
    }
    match conditions.is_empty() {
        true => String::new(),
        false => format!(" WHERE {}", conditions.join(" AND ")),
    }
}

fn bind_filters<'a>(
    mut query: diesel::query_builder::BoxedSqlQuery<
        'a,
        Sqlite,
        diesel::query_builder::SqlQuery,
    >,
    trigger: &Option<WebHookTrigger>,
    status: &Option<WebHookExecutionStatus>,
) -> diesel::query_builder::BoxedSqlQuery<
    'a,
    Sqlite,
    diesel::query_builder::SqlQuery,
> {
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
        id: Uuid::parse_str(&row.id).map_err(|e| {
            fetching_err(format!("Invalid webhook execution id: {e}"))
        })?,
        trigger: WebHookTrigger::from_str(&row.trigger).map_err(|e| {
            fetching_err(format!("Invalid webhook trigger: {e}"))
        })?,
        payload_id: row.payload_id,
        dispatch: WebHookDispatchState {
            status: WebHookExecutionStatus::from_str(&row.status)?,
            attempts: row.attempts.max(0) as u32,
            created: local(naive_timestamp_from_text(&row.created)?),
            attempted: row
                .attempted
                .map(|value| naive_timestamp_from_text(&value).map(local))
                .transpose()?,
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
        latest: local(naive_timestamp_from_text(&row.latest)?),
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
        let filter = where_clause(&trigger, &status);

        let rows = bind_filters(
            diesel::sql_query(format!(
                "SELECT * FROM webhook_execution_event{filter} \
                 ORDER BY created DESC LIMIT ? OFFSET ?"
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
            if since.is_some() { " WHERE created >= ?" } else { "" }
        ))
        .into_boxed::<Sqlite>();

        if let Some(since) = since {
            query = query.bind::<Text, _>(
                crate::types::naive_timestamp_to_text(&since.naive_utc()),
            );
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
    use crate::{
        repositories::webhook::{
            WebHookRegistrationSqlDbRepository, WebHookUpdatingSqlDbRepository,
        },
        test_support::setup_temp_db,
    };
    use myc_core::domain::{
        dtos::webhook::{HookResponse, PayloadId, WebHookPayloadArtifact},
        entities::{WebHookRegistration, WebHookUpdating},
    };
    use mycelium_base::entities::CreateResponseKind;

    async fn register(
        registration: &WebHookRegistrationSqlDbRepository,
        trigger: WebHookTrigger,
    ) -> Result<WebHookPayloadArtifact, MappedErrors> {
        let artifact = WebHookPayloadArtifact::new(
            None,
            "{\"secret\":\"do-not-leak\"}".into(),
            PayloadId::String("payload-1".into()),
            trigger,
        );
        let mut stored = artifact.clone();
        match registration.register_execution_event(artifact).await? {
            CreateResponseKind::Created(id) => stored.id = Some(id),
            CreateResponseKind::NotCreated(..) => panic!("not created"),
        }
        Ok(stored)
    }

    fn delivery(status: u16) -> HookResponse {
        HookResponse {
            url: "https://hooks.example.com/private?token=abc".into(),
            status,
            body: Some("sensitive response body".into()),
            datetime: Local::now(),
        }
    }

    #[tokio::test]
    async fn lists_events_with_delivery_counts_and_no_payload(
    ) -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = WebHookRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let updating = WebHookUpdatingSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = WebHookExecutionFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        let mut delivered =
            register(&registration, WebHookTrigger::SubscriptionAccountCreated)
                .await?;
        delivered.status = Some(WebHookExecutionStatus::Failed);
        delivered.attempts = Some(2);
        delivered.propagations = Some(vec![delivery(200), delivery(503)]);
        updating.update_execution_event(delivered.clone()).await?;

        register(&registration, WebHookTrigger::UserAccountCreated).await?;

        let FetchManyResponseKind::FoundPaginated { count, records, .. } =
            fetching.list_events(None, None, None, None).await?
        else {
            panic!("expected paginated events");
        };
        assert_eq!(count, 2);

        let event = records
            .iter()
            .find(|event| Some(event.id) == delivered.id)
            .expect("the delivered event is listed");
        assert_eq!(event.dispatch.status, WebHookExecutionStatus::Failed);
        assert_eq!(event.dispatch.attempts, 2);
        assert_eq!(event.propagations.total, 2);
        assert_eq!(event.propagations.failed, 1);
        assert_eq!(event.propagations.last_response_status, Some(503));

        // Nothing sensitive reaches the result
        let serialized = serde_json::to_string(&records).unwrap();
        assert!(!serialized.contains("do-not-leak"));
        assert!(!serialized.contains("hooks.example.com"));
        assert!(!serialized.contains("sensitive response body"));

        // A pending event without propagations counts zero deliveries
        let pending = records
            .iter()
            .find(|event| Some(event.id) != delivered.id)
            .expect("the pending event is listed");
        assert_eq!(pending.propagations.total, 0);
        assert_eq!(pending.propagations.last_response_status, None);

        Ok(())
    }

    #[tokio::test]
    async fn filters_by_trigger_and_status() -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = WebHookRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = WebHookExecutionFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        register(&registration, WebHookTrigger::SubscriptionAccountCreated)
            .await?;
        register(&registration, WebHookTrigger::UserAccountCreated).await?;

        let FetchManyResponseKind::FoundPaginated { count, records, .. } =
            fetching
                .list_events(
                    Some(WebHookTrigger::UserAccountCreated),
                    Some(WebHookExecutionStatus::Pending),
                    None,
                    None,
                )
                .await?
        else {
            panic!("expected the user-account event");
        };
        assert_eq!(count, 1);
        assert_eq!(records[0].trigger, WebHookTrigger::UserAccountCreated);

        let none = fetching
            .list_events(
                None,
                Some(WebHookExecutionStatus::Success),
                None,
                None,
            )
            .await?;
        assert!(matches!(none, FetchManyResponseKind::NotFound));

        // A filter value is data, never SQL
        let injected = fetching
            .list_events(
                None,
                Some(WebHookExecutionStatus::from_str("x' OR '1'='1")?),
                None,
                None,
            )
            .await?;
        assert!(matches!(injected, FetchManyResponseKind::NotFound));

        Ok(())
    }

    #[tokio::test]
    async fn summarizes_by_trigger_and_status() -> Result<(), MappedErrors> {
        let db = setup_temp_db();
        let registration = WebHookRegistrationSqlDbRepository {
            db_config: db.provider.clone(),
        };
        let fetching = WebHookExecutionFetchingSqlDbRepository {
            db_config: db.provider.clone(),
        };

        for _ in 0..3 {
            register(&registration, WebHookTrigger::SubscriptionAccountCreated)
                .await?;
        }
        register(&registration, WebHookTrigger::UserAccountCreated).await?;

        let FetchManyResponseKind::Found(summary) =
            fetching.summarize_events(None).await?
        else {
            panic!("expected a summary");
        };
        let subscription = summary
            .iter()
            .find(|s| s.trigger == WebHookTrigger::SubscriptionAccountCreated)
            .expect("subscription trigger summarized");
        assert_eq!(subscription.count, 3);
        assert_eq!(subscription.status, WebHookExecutionStatus::Pending);

        let future = fetching
            .summarize_events(Some(Local::now() + chrono::Duration::days(1)))
            .await?;
        assert!(matches!(future, FetchManyResponseKind::NotFound));

        Ok(())
    }
}
