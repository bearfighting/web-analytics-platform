use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{DateTime, NaiveDate, Utc};
use serde_json::json;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Postgres, Transaction, postgres::PgPoolOptions};

use crate::{
    ProcessorError,
    models::RawEvent,
    normalizer::{NormalizedContext, normalize_context},
    parser::{UserAgentParser, WOOTHEE_VERSION, WootheeParser},
    queries,
    sessionizer::{SessionInput, SessionOutput, sessionize},
};

const AGGREGATION_VERSION: i32 = 1;

#[derive(Clone)]
pub struct Processor {
    pool: PgPool,
}

#[derive(Debug, Clone)]
struct RebuildRequest {
    queue_id: Option<i64>,
    site_id: String,
    scope_from: NaiveDate,
    scope_to: NaiveDate,
    parser_version: String,
    rebuild_reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RebuildSummary {
    pub events: usize,
    pub visitors: usize,
    pub scope_from: NaiveDate,
    pub scope_to: NaiveDate,
}

impl Processor {
    pub async fn connect(database_url: &str) -> Result<Self, ProcessorError> {
        let pool = PgPoolOptions::new()
            .max_connections(5)
            .connect(database_url)
            .await?;
        Ok(Self { pool })
    }

    pub async fn process_one(&self) -> Result<bool, ProcessorError> {
        let mut transaction = self.pool.begin().await?;
        let Some(event) = queries::claim_next_event(&mut transaction).await? else {
            transaction.rollback().await?;
            return Ok(false);
        };

        process_event(&mut transaction, &event).await?;
        transaction.commit().await?;
        Ok(true)
    }

    pub async fn process_all_once(&self) -> Result<u64, ProcessorError> {
        let mut processed = 0;
        while self.process_one().await? {
            processed += 1;
        }
        while self.process_rebuild_queue_once().await? {}
        Ok(processed)
    }

    pub async fn process_rebuild_queue_once(&self) -> Result<bool, ProcessorError> {
        let mut transaction = self.pool.begin().await?;
        let request = sqlx::query_as::<_, (i64, String, NaiveDate, NaiveDate, String, String)>(
            "SELECT queue_id, site_id, scope_from, scope_to, parser_version, rebuild_reason
             FROM analytics_rebuild_queue
             WHERE rebuild_reason = 'incremental'
               AND (
                   status = 'pending'
                   OR (status = 'running' AND updated_at < NOW() - INTERVAL '5 minutes')
               )
             ORDER BY queue_id
             LIMIT 1
             FOR UPDATE SKIP LOCKED",
        )
        .fetch_optional(&mut *transaction)
        .await?;

        let Some((queue_id, site_id, scope_from, scope_to, parser_version, rebuild_reason)) =
            request
        else {
            transaction.rollback().await?;
            return Ok(false);
        };

        // Do not claim work while the site-level Phase 6 switch is disabled.
        // Keeping the row pending makes disabling the feature a safe pause,
        // rather than turning queued work into a failed rebuild.
        if !queries::phase6_enabled(&mut transaction, &site_id).await? {
            transaction.rollback().await?;
            return Ok(false);
        }

        sqlx::query(
            "UPDATE analytics_rebuild_queue
             SET status = 'running', attempts = attempts + 1, updated_at = NOW()
             WHERE queue_id = $1",
        )
        .bind(queue_id)
        .execute(&mut *transaction)
        .await?;
        transaction.commit().await?;

        let request = RebuildRequest {
            queue_id: Some(queue_id),
            site_id,
            scope_from,
            scope_to,
            parser_version,
            rebuild_reason,
        };
        match self.rebuild(request).await {
            Ok(()) => Ok(true),
            Err(ProcessorError::RebuildQueueAlreadyHandled(_)) => Ok(true),
            Err(ProcessorError::RebuildQueuePaused(_)) => Ok(false),
            Err(error) => {
                sqlx::query(
                    "UPDATE analytics_rebuild_queue
                     SET status = 'failed', failure_reason = $2, updated_at = NOW()
                     WHERE queue_id = $1",
                )
                .bind(queue_id)
                .bind(error.to_string())
                .execute(&self.pool)
                .await?;
                Err(error)
            }
        }
    }

    pub async fn enqueue_rebuild(
        &self,
        site_id: &str,
        scope_from: NaiveDate,
        scope_to: NaiveDate,
        reason: &str,
        parser_version: &str,
    ) -> Result<(), ProcessorError> {
        if scope_from > scope_to {
            return Err(ProcessorError::InvalidRebuildScope);
        }
        sqlx::query(
            "INSERT INTO analytics_rebuild_queue
                (site_id, scope_from, scope_to, aggregation_version,
                 parser_version, rebuild_reason)
             VALUES ($1, $2, $3, $4, $5, $6)
             ON CONFLICT (site_id, visitor_id, aggregation_version, scope_from, scope_to)
                 WHERE status IN ('pending', 'running')
             DO UPDATE SET
                 rebuild_reason = CASE
                     WHEN analytics_rebuild_queue.rebuild_reason = 'backfill'
                       OR EXCLUDED.rebuild_reason = 'backfill'
                     THEN 'backfill'
                     ELSE analytics_rebuild_queue.rebuild_reason
                 END,
                 parser_version = EXCLUDED.parser_version,
                 updated_at = NOW()",
        )
        .bind(site_id)
        .bind(scope_from)
        .bind(scope_to)
        .bind(AGGREGATION_VERSION)
        .bind(parser_version)
        .bind(reason)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn rebuild_site(
        &self,
        site_id: &str,
        scope_from: NaiveDate,
        scope_to: NaiveDate,
        reason: &str,
        parser_version: &str,
        dry_run: bool,
    ) -> Result<RebuildSummary, ProcessorError> {
        if scope_from > scope_to {
            return Err(ProcessorError::InvalidRebuildScope);
        }
        if parser_version != WOOTHEE_VERSION {
            return Err(ProcessorError::UnsupportedParserVersion(
                parser_version.to_owned(),
            ));
        }
        let events = load_site_events(&self.pool, site_id).await?;
        let summary = RebuildSummary {
            events: events.len(),
            visitors: events
                .iter()
                .filter_map(|event| event.visitor_id.as_deref())
                .collect::<HashSet<_>>()
                .len(),
            scope_from,
            scope_to,
        };
        if dry_run {
            return Ok(summary);
        }
        let enabled = sqlx::query_scalar::<_, bool>(
            "SELECT analytics_enabled FROM analytics_feature_flags WHERE site_id = $1",
        )
        .bind(site_id)
        .fetch_optional(&self.pool)
        .await?
        .unwrap_or(false);
        if !enabled {
            return Err(ProcessorError::AnalyticsDisabled {
                site_id: site_id.to_owned(),
            });
        }

        self.rebuild(RebuildRequest {
            queue_id: None,
            site_id: site_id.to_owned(),
            scope_from,
            scope_to,
            parser_version: parser_version.to_owned(),
            rebuild_reason: reason.to_owned(),
        })
        .await?;
        Ok(summary)
    }

    pub async fn rollback_generation(
        &self,
        site_id: &str,
        target_generation_id: &str,
    ) -> Result<(), ProcessorError> {
        let mut transaction = self.pool.begin().await?;
        let target_status = sqlx::query_scalar::<_, String>(
            "SELECT status
             FROM analytics_generations
             WHERE site_id = $1 AND generation_id = $2::uuid
             FOR UPDATE",
        )
        .bind(site_id)
        .bind(target_generation_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| ProcessorError::GenerationNotFound {
            site_id: site_id.to_owned(),
            generation_id: target_generation_id.to_owned(),
        })?;

        if target_status != "retired" {
            return Err(ProcessorError::InvalidRollbackTarget {
                generation_id: target_generation_id.to_owned(),
                status: target_status,
            });
        }

        let active_generation_id = sqlx::query_scalar::<_, String>(
            "SELECT generation_id::text
             FROM analytics_generations
             WHERE site_id = $1 AND status = 'active'
             FOR UPDATE",
        )
        .bind(site_id)
        .fetch_optional(&mut *transaction)
        .await?
        .ok_or_else(|| ProcessorError::NoActiveGeneration {
            site_id: site_id.to_owned(),
        })?;

        sqlx::query(
            "UPDATE analytics_generations
             SET status = 'retired'
             WHERE site_id = $1 AND generation_id = $2::uuid AND status = 'active'",
        )
        .bind(site_id)
        .bind(&active_generation_id)
        .execute(&mut *transaction)
        .await?;
        sqlx::query(
            "UPDATE analytics_generations
             SET status = 'active', activated_at = NOW(), failure_reason = NULL
             WHERE site_id = $1 AND generation_id = $2::uuid AND status = 'retired'",
        )
        .bind(site_id)
        .bind(target_generation_id)
        .execute(&mut *transaction)
        .await?;

        transaction.commit().await?;
        Ok(())
    }

    async fn rebuild(&self, request: RebuildRequest) -> Result<(), ProcessorError> {
        let generation_id = new_generation_id(&request.site_id, &request.parser_version);
        match self.rebuild_inner(request.clone(), &generation_id).await {
            Ok(()) => Ok(()),
            Err(error @ ProcessorError::RebuildQueueAlreadyHandled(_))
            | Err(error @ ProcessorError::RebuildQueuePaused(_)) => Err(error),
            Err(error) => {
                let _ = sqlx::query(
                    "INSERT INTO analytics_generations
                        (generation_id, site_id, aggregation_version, parser_version,
                         scope_from, scope_to, rebuild_reason, status, failure_reason)
                     VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, 'failed', $8)
                     ON CONFLICT (generation_id) DO NOTHING",
                )
                .bind(&generation_id)
                .bind(&request.site_id)
                .bind(AGGREGATION_VERSION)
                .bind(&request.parser_version)
                .bind(request.scope_from)
                .bind(request.scope_to)
                .bind(&request.rebuild_reason)
                .bind(error.to_string())
                .execute(&self.pool)
                .await;
                Err(error)
            }
        }
    }

    async fn rebuild_inner(
        &self,
        request: RebuildRequest,
        generation_id: &str,
    ) -> Result<(), ProcessorError> {
        let parser = WootheeParser::new();
        let mut transaction = self.pool.begin().await?;

        lock_site(&mut transaction, &request.site_id).await?;
        if let Some(queue_id) = request.queue_id {
            let queue_status = sqlx::query_scalar::<_, String>(
                "SELECT status FROM analytics_rebuild_queue
                 WHERE queue_id = $1
                 FOR UPDATE",
            )
            .bind(queue_id)
            .fetch_one(&mut *transaction)
            .await?;
            if queue_status == "completed" || queue_status == "failed" {
                return Err(ProcessorError::RebuildQueueAlreadyHandled(queue_id));
            }

            // Re-check after taking the site lock. The initial check happens
            // before claiming the queue; this one protects the generation
            // transaction if the flag is disabled between those operations.
            let enabled = sqlx::query_scalar::<_, bool>(
                "SELECT analytics_enabled
                 FROM analytics_feature_flags
                 WHERE site_id = $1
                 FOR SHARE",
            )
            .bind(&request.site_id)
            .fetch_optional(&mut *transaction)
            .await?
            .unwrap_or(false);
            if !enabled {
                sqlx::query(
                    "UPDATE analytics_rebuild_queue
                     SET status = 'pending', failure_reason = NULL, updated_at = NOW()
                     WHERE queue_id = $1",
                )
                .bind(queue_id)
                .execute(&mut *transaction)
                .await?;
                transaction.commit().await?;
                return Err(ProcessorError::RebuildQueuePaused(queue_id));
            }
        }
        let events = load_site_events(&self.pool, &request.site_id).await?;
        let watermark = match events.iter().map(|event| event.received_at).max() {
            Some(max_received_at) => {
                Some(continuous_watermark(&self.pool, &request.site_id, max_received_at).await?)
            }
            None => None,
        };

        sqlx::query(
            "INSERT INTO analytics_generations
                (generation_id, site_id, aggregation_version, parser_version,
                 scope_from, scope_to, rebuild_reason, status, source_watermark)
             VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, 'building', $8)",
        )
        .bind(generation_id)
        .bind(&request.site_id)
        .bind(AGGREGATION_VERSION)
        .bind(&request.parser_version)
        .bind(request.scope_from)
        .bind(request.scope_to)
        .bind(&request.rebuild_reason)
        .bind(json!({
            "visitor_session": watermark.map(|value| value.to_rfc3339())
        }))
        .execute(&mut *transaction)
        .await?;

        write_derived_results(&mut transaction, generation_id, &events, &parser).await?;

        if let Some(watermark) = watermark {
            sqlx::query(
                "INSERT INTO analytics_watermarks
                    (site_id, generation_id, source_name, processed_received_watermark)
                 VALUES ($1, $2::uuid, 'visitor_session', $3)
                 ON CONFLICT (site_id, generation_id, source_name)
                 DO UPDATE SET processed_received_watermark = EXCLUDED.processed_received_watermark,
                               updated_at = NOW()",
            )
            .bind(&request.site_id)
            .bind(generation_id)
            .bind(watermark)
            .execute(&mut *transaction)
            .await?;
        }

        let active_generation = sqlx::query_scalar::<_, String>(
            "SELECT generation_id::text
             FROM analytics_generations
             WHERE site_id = $1 AND status = 'active'
             FOR UPDATE",
        )
        .bind(&request.site_id)
        .fetch_optional(&mut *transaction)
        .await?;
        if let Some(active_generation) = active_generation {
            sqlx::query(
                "UPDATE analytics_generations
                 SET status = 'retired'
                 WHERE site_id = $1 AND generation_id = $2::uuid",
            )
            .bind(&request.site_id)
            .bind(active_generation)
            .execute(&mut *transaction)
            .await?;
        }

        sqlx::query(
            "UPDATE analytics_generations
             SET status = 'active', activated_at = NOW()
             WHERE generation_id = $1::uuid",
        )
        .bind(generation_id)
        .execute(&mut *transaction)
        .await?;
        if let Some(queue_id) = request.queue_id {
            sqlx::query(
                "UPDATE analytics_rebuild_queue
                 SET status = 'completed', updated_at = NOW()
                 WHERE queue_id = $1",
            )
            .bind(queue_id)
            .execute(&mut *transaction)
            .await?;
        } else if request.rebuild_reason == "backfill" {
            sqlx::query(
                "UPDATE analytics_rebuild_queue
                 SET status = 'completed', updated_at = NOW()
                 WHERE site_id = $1
                   AND scope_from = $2
                   AND scope_to = $3
                   AND rebuild_reason = 'backfill'
                   AND status IN ('pending', 'running')",
            )
            .bind(&request.site_id)
            .bind(request.scope_from)
            .bind(request.scope_to)
            .execute(&mut *transaction)
            .await?;
        }

        transaction.commit().await?;
        Ok(())
    }
}

async fn continuous_watermark(
    pool: &PgPool,
    site_id: &str,
    max_received_at: DateTime<Utc>,
) -> Result<DateTime<Utc>, ProcessorError> {
    let first_unprocessed = sqlx::query_scalar::<_, Option<DateTime<Utc>>>(
        "SELECT MIN(received_at)
         FROM raw_events
         WHERE site_id = $1
           AND processed_at IS NULL
           AND received_at <= $2",
    )
    .bind(site_id)
    .bind(max_received_at)
    .fetch_one(pool)
    .await?;
    Ok(match first_unprocessed {
        Some(gap) => gap - chrono::Duration::microseconds(1),
        None => max_received_at,
    })
}

async fn process_event(
    transaction: &mut Transaction<'_, Postgres>,
    event: &RawEvent,
) -> Result<(), ProcessorError> {
    let day = event.occurred_at.with_timezone(&Utc).date_naive();
    queries::upsert_daily(transaction, &event.site_id, day).await?;
    queries::upsert_route(transaction, &event.site_id, day, &event.path).await?;
    queries::upsert_total(transaction, &event.site_id).await?;

    if queries::phase6_enabled(transaction, &event.site_id).await? {
        lock_site(transaction, &event.site_id).await?;
        if let Some(visitor_id) = event.visitor_id.as_deref() {
            let delay = event.received_at.signed_duration_since(event.occurred_at);
            let rebuild_reason = if delay > chrono::Duration::hours(24) {
                "backfill"
            } else {
                "incremental"
            };
            queries::enqueue_rebuild(
                transaction,
                &event.site_id,
                visitor_id,
                day,
                rebuild_reason,
                WOOTHEE_VERSION,
            )
            .await?;
        }
    }

    if !queries::mark_processed(transaction, event.id).await? {
        return Err(ProcessorError::RawEventNotUpdated(event.id));
    }
    Ok(())
}

async fn lock_site(
    transaction: &mut Transaction<'_, Postgres>,
    site_id: &str,
) -> Result<(), ProcessorError> {
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(site_id)
        .execute(&mut **transaction)
        .await?;
    Ok(())
}

async fn load_site_events(pool: &PgPool, site_id: &str) -> Result<Vec<RawEvent>, ProcessorError> {
    Ok(sqlx::query_as::<
        _,
        (
            i64,
            String,
            DateTime<Utc>,
            DateTime<Utc>,
            String,
            i32,
            Option<String>,
            Option<i32>,
            serde_json::Value,
        ),
    >(
        "SELECT id, event_id, occurred_at, received_at, path, schema_version,
                visitor_id::text, context_schema_version, payload
         FROM raw_events
         WHERE site_id = $1 AND processed_at IS NOT NULL
         ORDER BY occurred_at, event_id",
    )
    .bind(site_id)
    .fetch_all(pool)
    .await?
    .into_iter()
    .map(
        |(
            id,
            event_id,
            occurred_at,
            received_at,
            path,
            schema_version,
            visitor_id,
            context_schema_version,
            payload,
        )| RawEvent {
            id,
            event_id,
            site_id: site_id.to_owned(),
            occurred_at,
            received_at,
            path,
            schema_version,
            visitor_id,
            context_schema_version,
            payload,
        },
    )
    .collect())
}

async fn write_derived_results(
    transaction: &mut Transaction<'_, Postgres>,
    generation_id: &str,
    events: &[RawEvent],
    parser: &impl UserAgentParser,
) -> Result<(), ProcessorError> {
    for event in events
        .iter()
        .filter(|event| event.schema_version == 2 && event.context_schema_version.unwrap_or(1) == 1)
    {
        let context = event.payload.get("context");
        let user_agent = context
            .and_then(|value| value.get("user_agent"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("unknown");
        let normalized = normalize_context(context, user_agent, parser);
        insert_normalized_context(transaction, generation_id, event, &normalized).await?;
    }

    let visitor_events: Vec<&RawEvent> = events
        .iter()
        .filter(|event| event.visitor_id.is_some())
        .collect();
    for event in &visitor_events {
        let visitor_id = event.visitor_id.as_deref().expect("filtered above");
        sqlx::query(
            "INSERT INTO visitor_event_facts
                (generation_id, raw_event_id, site_id, visitor_id, occurred_at, day)
             VALUES ($1::uuid, $2, $3, $4::uuid, $5, $6)",
        )
        .bind(generation_id)
        .bind(event.id)
        .bind(&event.site_id)
        .bind(visitor_id)
        .bind(event.occurred_at)
        .bind(event.occurred_at.date_naive())
        .execute(&mut **transaction)
        .await?;
    }

    let mut grouped: HashMap<(String, String), Vec<SessionInput>> = HashMap::new();
    for event in &visitor_events {
        let visitor_id = event.visitor_id.as_deref().expect("filtered above");
        grouped
            .entry((event.site_id.clone(), visitor_id.to_owned()))
            .or_default()
            .push(SessionInput {
                event_id: event.event_id.clone(),
                site_id: event.site_id.clone(),
                visitor_id: visitor_id.to_owned(),
                occurred_at: event.occurred_at,
            });
    }

    let mut sessions: Vec<SessionOutput> = Vec::new();
    for group in grouped.values() {
        sessions.extend(sessionize(group, generation_id));
    }
    let event_lookup: HashMap<&str, &RawEvent> = visitor_events
        .iter()
        .map(|event| (event.event_id.as_str(), *event))
        .collect();

    for session in &sessions {
        sqlx::query(
            "INSERT INTO sessions
                (generation_id, session_id, site_id, visitor_id, started_at, ended_at, page_views)
             VALUES ($1::uuid, $2::uuid, $3, $4::uuid, $5, $6, $7)",
        )
        .bind(generation_id)
        .bind(&session.session_id)
        .bind(&session.site_id)
        .bind(&session.visitor_id)
        .bind(session.started_at)
        .bind(session.ended_at)
        .bind(session.page_views)
        .execute(&mut **transaction)
        .await?;

        for session_event in &session.events {
            let event = event_lookup
                .get(session_event.event_id.as_str())
                .expect("session event must reference raw event");
            sqlx::query(
                "INSERT INTO session_events
                    (generation_id, raw_event_id, site_id, visitor_id, session_id, occurred_at, day)
                 VALUES ($1::uuid, $2, $3, $4::uuid, $5::uuid, $6, $7)",
            )
            .bind(generation_id)
            .bind(event.id)
            .bind(&event.site_id)
            .bind(&session.visitor_id)
            .bind(&session.session_id)
            .bind(event.occurred_at)
            .bind(event.occurred_at.date_naive())
            .execute(&mut **transaction)
            .await?;
        }
    }

    let mut visitor_daily: BTreeMap<(String, NaiveDate), (HashSet<String>, i64)> = BTreeMap::new();
    for event in &visitor_events {
        let entry = visitor_daily
            .entry((event.site_id.clone(), event.occurred_at.date_naive()))
            .or_default();
        entry
            .0
            .insert(event.visitor_id.clone().expect("filtered above"));
        entry.1 += 1;
    }
    for ((site_id, day), (unique_visitors, page_views)) in visitor_daily {
        sqlx::query(
            "INSERT INTO visitor_daily
                (generation_id, site_id, day, unique_visitors, page_views)
             VALUES ($1::uuid, $2, $3, $4, $5)",
        )
        .bind(generation_id)
        .bind(site_id)
        .bind(day)
        .bind(unique_visitors.len() as i64)
        .bind(page_views)
        .execute(&mut **transaction)
        .await?;
    }

    let mut session_daily: BTreeMap<(String, NaiveDate), (i64, i64)> = BTreeMap::new();
    for session in &sessions {
        let entry = session_daily
            .entry((session.site_id.clone(), session.started_at.date_naive()))
            .or_default();
        entry.0 += 1;
        entry.1 += session.page_views;
    }
    for ((site_id, day), (session_count, page_views)) in session_daily {
        sqlx::query(
            "INSERT INTO session_daily
                (generation_id, site_id, day, sessions, page_views)
             VALUES ($1::uuid, $2, $3, $4, $5)",
        )
        .bind(generation_id)
        .bind(site_id)
        .bind(day)
        .bind(session_count)
        .bind(page_views)
        .execute(&mut **transaction)
        .await?;
    }
    Ok(())
}

async fn insert_normalized_context(
    transaction: &mut Transaction<'_, Postgres>,
    generation_id: &str,
    event: &RawEvent,
    context: &NormalizedContext,
) -> Result<(), ProcessorError> {
    sqlx::query(
        "INSERT INTO normalized_event_context
            (generation_id, raw_event_id, site_id, context_schema_version,
             parser_version, language, timezone, viewport_width, viewport_height,
             screen_width, screen_height, utm_source, utm_medium, utm_campaign,
             utm_term, utm_content, referrer_host, device, browser, os)
         VALUES ($1::uuid, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11,
                 $12, $13, $14, $15, $16, $17, $18, $19, $20)",
    )
    .bind(generation_id)
    .bind(event.id)
    .bind(&event.site_id)
    .bind(context.context_schema_version)
    .bind(WOOTHEE_VERSION)
    .bind(&context.language)
    .bind(&context.timezone)
    .bind(context.viewport_width)
    .bind(context.viewport_height)
    .bind(context.screen_width)
    .bind(context.screen_height)
    .bind(&context.utm_source)
    .bind(&context.utm_medium)
    .bind(&context.utm_campaign)
    .bind(&context.utm_term)
    .bind(&context.utm_content)
    .bind(&context.referrer_host)
    .bind(&context.device)
    .bind(&context.browser)
    .bind(&context.os)
    .execute(&mut **transaction)
    .await?;
    Ok(())
}

fn new_generation_id(site_id: &str, parser_version: &str) -> String {
    let input = format!(
        "phase6-generation\0{site_id}\0{parser_version}\0{}",
        Utc::now()
    );
    let digest = Sha256::digest(input.as_bytes());
    let mut bytes = [0_u8; 16];
    bytes.copy_from_slice(&digest[..16]);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0],
        bytes[1],
        bytes[2],
        bytes[3],
        bytes[4],
        bytes[5],
        bytes[6],
        bytes[7],
        bytes[8],
        bytes[9],
        bytes[10],
        bytes[11],
        bytes[12],
        bytes[13],
        bytes[14],
        bytes[15]
    )
}
