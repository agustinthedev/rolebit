use std::{fs, path::PathBuf};

use chrono::{Duration, Utc};
use rusqlite::{
    Connection, OptionalExtension, Row, Transaction, TransactionBehavior, params, params_from_iter,
    types::Value,
};
use sha2::{Digest, Sha256};

use crate::models::{
    ActivityDay, AppSettings, CandidateJob, Diagnostics, IngestionRun, IngestionRunDetail,
    JobDetail, JobDiscovery, JobFacets, JobListRequest, JobPage, JobSummary, QueryExecution,
    SaveSettingsInput, SearchConfiguration, SearchItem,
};

const MIGRATION_1: &str = r#"
CREATE TABLE IF NOT EXISTS schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS application_settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS search_items (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  kind TEXT NOT NULL CHECK(kind IN ('site', 'location', 'role')),
  value TEXT NOT NULL,
  enabled INTEGER NOT NULL DEFAULT 1 CHECK(enabled IN (0, 1)),
  sort_order INTEGER NOT NULL,
  UNIQUE(kind, value)
);

CREATE TABLE IF NOT EXISTS search_queries (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  query TEXT NOT NULL UNIQUE,
  active INTEGER NOT NULL DEFAULT 1 CHECK(active IN (0, 1)),
  created_at TEXT NOT NULL,
  last_used_at TEXT
);

CREATE TABLE IF NOT EXISTS ingestion_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  trigger_type TEXT NOT NULL,
  status TEXT NOT NULL,
  total_queries INTEGER NOT NULL DEFAULT 0,
  attempted_requests INTEGER NOT NULL DEFAULT 0,
  successful_queries INTEGER NOT NULL DEFAULT 0,
  failed_queries INTEGER NOT NULL DEFAULT 0,
  urls_discovered INTEGER NOT NULL DEFAULT 0,
  new_jobs INTEGER NOT NULL DEFAULT 0,
  updated_jobs INTEGER NOT NULL DEFAULT 0,
  extraction_failures INTEGER NOT NULL DEFAULT 0,
  estimated_cost REAL,
  error_summary TEXT
);

CREATE TABLE IF NOT EXISTS query_executions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id INTEGER NOT NULL REFERENCES ingestion_runs(id) ON DELETE CASCADE,
  query_id INTEGER NOT NULL REFERENCES search_queries(id),
  status TEXT NOT NULL,
  result_count INTEGER NOT NULL DEFAULT 0,
  candidate_count INTEGER NOT NULL DEFAULT 0,
  new_jobs INTEGER NOT NULL DEFAULT 0,
  updated_jobs INTEGER NOT NULL DEFAULT 0,
  error TEXT,
  started_at TEXT NOT NULL,
  completed_at TEXT
);

CREATE TABLE IF NOT EXISTS jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_key TEXT NOT NULL UNIQUE,
  canonical_url TEXT NOT NULL UNIQUE,
  external_id TEXT,
  source_domain TEXT NOT NULL,
  title TEXT NOT NULL,
  company TEXT,
  description TEXT,
  location TEXT,
  remote_status TEXT NOT NULL DEFAULT 'unknown',
  employment_type TEXT,
  published_at TEXT,
  expires_at TEXT,
  lifecycle_status TEXT NOT NULL DEFAULT 'unknown',
  tracking_status TEXT NOT NULL DEFAULT 'new',
  extraction_status TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  content_updated INTEGER NOT NULL DEFAULT 0 CHECK(content_updated IN (0, 1)),
  reopened INTEGER NOT NULL DEFAULT 0 CHECK(reopened IN (0, 1))
);

CREATE TABLE IF NOT EXISTS discovery_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id INTEGER NOT NULL REFERENCES ingestion_runs(id) ON DELETE CASCADE,
  query_execution_id INTEGER NOT NULL REFERENCES query_executions(id) ON DELETE CASCADE,
  job_id INTEGER REFERENCES jobs(id) ON DELETE SET NULL,
  provider TEXT NOT NULL,
  discovered_url TEXT NOT NULL,
  result_title TEXT,
  snippet TEXT,
  status TEXT NOT NULL,
  error TEXT,
  discovered_at TEXT NOT NULL,
  UNIQUE(run_id, query_execution_id, discovered_url)
);

CREATE TABLE IF NOT EXISTS job_discoveries (
  job_id INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  discovery_id INTEGER NOT NULL REFERENCES discovery_results(id) ON DELETE CASCADE,
  PRIMARY KEY(job_id, discovery_id)
);

CREATE TABLE IF NOT EXISTS job_sources (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  provider TEXT NOT NULL,
  source_url TEXT NOT NULL,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  UNIQUE(job_id, provider, source_url)
);

CREATE TABLE IF NOT EXISTS extraction_attempts (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  discovery_id INTEGER NOT NULL REFERENCES discovery_results(id) ON DELETE CASCADE,
  status TEXT NOT NULL,
  strategy TEXT,
  error TEXT,
  attempted_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS job_revisions (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  job_id INTEGER NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
  run_id INTEGER REFERENCES ingestion_runs(id) ON DELETE SET NULL,
  content_hash TEXT NOT NULL,
  snapshot_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_search_items_kind_order ON search_items(kind, sort_order);
CREATE INDEX IF NOT EXISTS idx_runs_started ON ingestion_runs(started_at DESC);
CREATE INDEX IF NOT EXISTS idx_query_executions_run ON query_executions(run_id);
CREATE INDEX IF NOT EXISTS idx_discoveries_job ON discovery_results(job_id);
CREATE INDEX IF NOT EXISTS idx_discoveries_run ON discovery_results(run_id);
CREATE INDEX IF NOT EXISTS idx_jobs_first_seen ON jobs(first_seen_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_last_seen ON jobs(last_seen_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_updated ON jobs(updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_jobs_status ON jobs(tracking_status, lifecycle_status);
CREATE INDEX IF NOT EXISTS idx_jobs_location ON jobs(location);
CREATE INDEX IF NOT EXISTS idx_jobs_source ON jobs(source_domain);

CREATE VIRTUAL TABLE IF NOT EXISTS jobs_fts USING fts5(
  title, company, description, content='jobs', content_rowid='id'
);

CREATE TRIGGER IF NOT EXISTS jobs_ai AFTER INSERT ON jobs BEGIN
  INSERT INTO jobs_fts(rowid, title, company, description)
  VALUES (new.id, new.title, new.company, new.description);
END;
CREATE TRIGGER IF NOT EXISTS jobs_ad AFTER DELETE ON jobs BEGIN
  INSERT INTO jobs_fts(jobs_fts, rowid, title, company, description)
  VALUES ('delete', old.id, old.title, old.company, old.description);
END;
CREATE TRIGGER IF NOT EXISTS jobs_au AFTER UPDATE ON jobs BEGIN
  INSERT INTO jobs_fts(jobs_fts, rowid, title, company, description)
  VALUES ('delete', old.id, old.title, old.company, old.description);
  INSERT INTO jobs_fts(rowid, title, company, description)
  VALUES (new.id, new.title, new.company, new.description);
END;
"#;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpsertOutcome {
    New,
    Updated,
    Existing,
}

#[derive(Clone)]
pub struct Database {
    path: PathBuf,
}

impl Database {
    pub fn initialize(path: PathBuf) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let database = Self { path };
        let mut connection = database.connect()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute_batch(MIGRATION_1)
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO schema_migrations(version, applied_at) VALUES(1, ?1)",
                [now()],
            )
            .map_err(|error| error.to_string())?;
        seed_defaults(&transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(database)
    }

    fn connect(&self) -> Result<Connection, String> {
        let connection = Connection::open(&self.path).map_err(|error| error.to_string())?;
        connection
            .busy_timeout(std::time::Duration::from_secs(10))
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(
                "PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;",
            )
            .map_err(|error| error.to_string())?;
        Ok(connection)
    }

    pub fn get_configuration(&self) -> Result<SearchConfiguration, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(
                "SELECT id, kind, value, enabled FROM search_items ORDER BY kind, sort_order, id",
            )
            .map_err(|error| error.to_string())?;
        let rows = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, bool>(3)?,
                ))
            })
            .map_err(|error| error.to_string())?;
        let mut sites = Vec::new();
        let mut locations = Vec::new();
        let mut roles = Vec::new();
        for row in rows {
            let (id, kind, value, enabled) = row.map_err(|error| error.to_string())?;
            let item = SearchItem {
                id: Some(id),
                value,
                enabled,
            };
            match kind.as_str() {
                "site" => sites.push(item),
                "location" => locations.push(item),
                "role" => roles.push(item),
                _ => {}
            }
        }
        let pages_per_query = setting_i64(&connection, "pages_per_query", 1)? as u32;
        Ok(SearchConfiguration {
            sites,
            locations,
            roles,
            pages_per_query,
        })
    }

    pub fn save_configuration(&self, config: &SearchConfiguration) -> Result<(), String> {
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        transaction
            .execute("DELETE FROM search_items", [])
            .map_err(|error| error.to_string())?;
        insert_items(&transaction, "site", &config.sites)?;
        insert_items(&transaction, "location", &config.locations)?;
        insert_items(&transaction, "role", &config.roles)?;
        set_setting(
            &transaction,
            "pages_per_query",
            &config.pages_per_query.to_string(),
        )?;
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn get_settings(
        &self,
        app_version: &str,
        has_api_key: bool,
    ) -> Result<AppSettings, String> {
        let connection = self.connect()?;
        Ok(AppSettings {
            onboarding_complete: setting_bool(&connection, "onboarding_complete", false)?,
            provider: setting(&connection, "provider", "mock")?,
            serp_zone: setting(&connection, "serp_zone", "")?,
            google_gl: setting(&connection, "google_gl", "us")?,
            google_hl: setting(&connection, "google_hl", "en")?,
            request_concurrency: setting_i64(&connection, "request_concurrency", 3)? as u32,
            request_timeout_seconds: setting_i64(&connection, "request_timeout_seconds", 30)?
                as u32,
            retry_count: setting_i64(&connection, "retry_count", 2)? as u32,
            pages_per_query: setting_i64(&connection, "pages_per_query", 1)? as u32,
            estimated_cost_per_request: setting_optional_f64(
                &connection,
                "estimated_cost_per_request",
            )?,
            request_budget: setting_optional_i64(&connection, "request_budget")?
                .map(|value| value as u32),
            schedule_enabled: setting_bool(&connection, "schedule_enabled", false)?,
            schedule_interval_hours: setting_i64(&connection, "schedule_interval_hours", 12)?
                as u32,
            run_on_startup: setting_bool(&connection, "run_on_startup", false)?,
            start_with_windows: setting_bool(&connection, "start_with_windows", false)?,
            close_to_tray: setting_bool(&connection, "close_to_tray", true)?,
            last_scheduled_run_at: setting_optional(&connection, "last_scheduled_run_at")?,
            has_api_key,
            database_path: self.path.to_string_lossy().to_string(),
            app_version: app_version.to_string(),
        })
    }

    pub fn save_settings(&self, input: &SaveSettingsInput) -> Result<(), String> {
        if input.provider != "mock" && input.provider != "bright_data" {
            return Err("Unsupported search provider".into());
        }
        if input.schedule_interval_hours == 0
            || input.request_timeout_seconds == 0
            || input.request_concurrency == 0
        {
            return Err("Intervals, timeout, and concurrency must be greater than zero".into());
        }
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction()
            .map_err(|error| error.to_string())?;
        let values = [
            ("provider", input.provider.clone()),
            ("serp_zone", input.serp_zone.trim().to_string()),
            ("google_gl", input.google_gl.trim().to_lowercase()),
            ("google_hl", input.google_hl.trim().to_lowercase()),
            ("request_concurrency", input.request_concurrency.to_string()),
            (
                "request_timeout_seconds",
                input.request_timeout_seconds.to_string(),
            ),
            ("retry_count", input.retry_count.to_string()),
            ("pages_per_query", input.pages_per_query.max(1).to_string()),
            (
                "estimated_cost_per_request",
                input
                    .estimated_cost_per_request
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            (
                "request_budget",
                input
                    .request_budget
                    .map(|v| v.to_string())
                    .unwrap_or_default(),
            ),
            ("schedule_enabled", input.schedule_enabled.to_string()),
            (
                "schedule_interval_hours",
                input.schedule_interval_hours.to_string(),
            ),
            ("run_on_startup", input.run_on_startup.to_string()),
            ("start_with_windows", input.start_with_windows.to_string()),
            ("close_to_tray", input.close_to_tray.to_string()),
        ];
        for (key, value) in values {
            set_setting(&transaction, key, &value)?;
        }
        transaction.commit().map_err(|error| error.to_string())
    }

    pub fn mark_onboarding_complete(&self) -> Result<(), String> {
        let connection = self.connect()?;
        set_setting(&connection, "onboarding_complete", "true")
    }

    pub fn set_last_scheduled_run(&self) -> Result<(), String> {
        let connection = self.connect()?;
        set_setting(&connection, "last_scheduled_run_at", &now())
    }

    pub fn set_schedule_enabled(&self, enabled: bool) -> Result<(), String> {
        let connection = self.connect()?;
        set_setting(&connection, "schedule_enabled", &enabled.to_string())
    }

    pub fn start_run(
        &self,
        trigger_type: &str,
        total_queries: usize,
        estimated_cost: Option<f64>,
    ) -> Result<i64, String> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO ingestion_runs(started_at, trigger_type, status, total_queries, estimated_cost) VALUES(?1, ?2, 'running', ?3, ?4)",
            params![now(), trigger_type, total_queries as i64, estimated_cost],
        ).map_err(|error| error.to_string())?;
        Ok(connection.last_insert_rowid())
    }

    pub fn start_query_execution(&self, run_id: i64, query: &str) -> Result<i64, String> {
        let connection = self.connect()?;
        let timestamp = now();
        connection.execute(
            "INSERT INTO search_queries(query, active, created_at, last_used_at) VALUES(?1, 1, ?2, ?2)
             ON CONFLICT(query) DO UPDATE SET active=1, last_used_at=excluded.last_used_at",
            params![query, timestamp],
        ).map_err(|error| error.to_string())?;
        let query_id: i64 = connection
            .query_row(
                "SELECT id FROM search_queries WHERE query=?1",
                [query],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        connection.execute(
            "INSERT INTO query_executions(run_id, query_id, status, started_at) VALUES(?1, ?2, 'running', ?3)",
            params![run_id, query_id, timestamp],
        ).map_err(|error| error.to_string())?;
        Ok(connection.last_insert_rowid())
    }

    pub fn record_request_attempt(&self, run_id: i64) -> Result<(), String> {
        let connection = self.connect()?;
        connection
            .execute(
                "UPDATE ingestion_runs SET attempted_requests=attempted_requests+1 WHERE id=?1",
                [run_id],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn finish_query_execution(
        &self,
        execution_id: i64,
        status: &str,
        result_count: usize,
        candidates: usize,
        new_jobs: usize,
        updated_jobs: usize,
        error: Option<&str>,
    ) -> Result<(), String> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE query_executions SET status=?2, result_count=?3, candidate_count=?4, new_jobs=?5, updated_jobs=?6, error=?7, completed_at=?8 WHERE id=?1",
            params![execution_id, status, result_count as i64, candidates as i64, new_jobs as i64, updated_jobs as i64, error, now()],
        ).map_err(|db_error| db_error.to_string())?;
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub fn record_failed_discovery(
        &self,
        run_id: i64,
        execution_id: i64,
        provider: &str,
        url: &str,
        title: Option<&str>,
        snippet: Option<&str>,
        error: &str,
    ) -> Result<(), String> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT OR IGNORE INTO discovery_results(run_id, query_execution_id, provider, discovered_url, result_title, snippet, status, error, discovered_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'extraction_failed', ?7, ?8)",
            params![run_id, execution_id, provider, url, title, snippet, error, now()],
        ).map_err(|db_error| db_error.to_string())?;
        let discovery_id: i64 = connection.query_row(
            "SELECT id FROM discovery_results WHERE run_id=?1 AND query_execution_id=?2 AND discovered_url=?3",
            params![run_id, execution_id, url],
            |row| row.get(0),
        ).map_err(|db_error| db_error.to_string())?;
        connection.execute(
            "INSERT INTO extraction_attempts(discovery_id, status, strategy, error, attempted_at) VALUES(?1, 'failed', 'automatic', ?2, ?3)",
            params![discovery_id, error, now()],
        ).map_err(|db_error| db_error.to_string())?;
        connection
            .execute(
                "UPDATE ingestion_runs SET extraction_failures=extraction_failures+1 WHERE id=?1",
                [run_id],
            )
            .map_err(|db_error| db_error.to_string())?;
        Ok(())
    }

    pub fn record_candidate(
        &self,
        run_id: i64,
        execution_id: i64,
        provider: &str,
        result_title: Option<&str>,
        snippet: Option<&str>,
        candidate: &CandidateJob,
    ) -> Result<UpsertOutcome, String> {
        let mut connection = self.connect()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|error| error.to_string())?;
        let timestamp = now();
        transaction.execute(
            "INSERT OR IGNORE INTO discovery_results(run_id, query_execution_id, provider, discovered_url, result_title, snippet, status, discovered_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, 'verified', ?7)",
            params![run_id, execution_id, provider, candidate.source_url, result_title, snippet, timestamp],
        ).map_err(|error| error.to_string())?;
        let discovery_id: i64 = transaction.query_row(
            "SELECT id FROM discovery_results WHERE run_id=?1 AND query_execution_id=?2 AND discovered_url=?3",
            params![run_id, execution_id, candidate.source_url],
            |row| row.get(0),
        ).map_err(|error| error.to_string())?;

        let source_key = candidate
            .external_id
            .as_ref()
            .map(|external_id| format!("{}:{external_id}", candidate.source_domain))
            .unwrap_or_else(|| candidate.canonical_url.clone());
        let hash = candidate_hash(candidate);
        let existing: Option<(i64, String, String)> = transaction.query_row(
            "SELECT id, content_hash, lifecycle_status FROM jobs WHERE source_key=?1 OR canonical_url=?2 LIMIT 1",
            params![source_key, candidate.canonical_url],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        ).optional().map_err(|error| error.to_string())?;

        let (job_id, outcome) = if let Some((job_id, old_hash, lifecycle)) = existing {
            let changed = old_hash != hash;
            if changed {
                let snapshot = serde_json::to_string(candidate).unwrap_or_else(|_| "{}".into());
                transaction.execute(
                    "INSERT INTO job_revisions(job_id, run_id, content_hash, snapshot_json, created_at) VALUES(?1, ?2, ?3, ?4, ?5)",
                    params![job_id, run_id, hash, snapshot, timestamp],
                ).map_err(|error| error.to_string())?;
            }
            let reopened = lifecycle == "closed";
            transaction.execute(
                "UPDATE jobs SET canonical_url=?2, external_id=COALESCE(?3, external_id), source_domain=?4,
                 title=?5, company=COALESCE(?6, company), description=COALESCE(?7, description),
                 location=COALESCE(?8, location), remote_status=?9, employment_type=COALESCE(?10, employment_type),
                 published_at=COALESCE(?11, published_at), expires_at=COALESCE(?12, expires_at),
                 lifecycle_status=CASE WHEN ?13 THEN 'active' ELSE lifecycle_status END,
                 extraction_status=?14, content_hash=?15, last_seen_at=?16,
                 updated_at=CASE WHEN ?17 THEN ?16 ELSE updated_at END,
                 content_updated=?17, reopened=?13 WHERE id=?1",
                params![
                    job_id, candidate.canonical_url, candidate.external_id, candidate.source_domain,
                    candidate.title, candidate.company, candidate.description, candidate.location,
                    candidate.remote_status, candidate.employment_type, candidate.published_at,
                    candidate.expires_at, reopened, candidate.extraction_status, hash, timestamp, changed,
                ],
            ).map_err(|error| error.to_string())?;
            (
                job_id,
                if changed {
                    UpsertOutcome::Updated
                } else {
                    UpsertOutcome::Existing
                },
            )
        } else {
            transaction.execute(
                "INSERT INTO jobs(source_key, canonical_url, external_id, source_domain, title, company, description,
                 location, remote_status, employment_type, published_at, expires_at, lifecycle_status,
                 tracking_status, extraction_status, content_hash, first_seen_at, last_seen_at, updated_at)
                 VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, 'active', 'new', ?13, ?14, ?15, ?15, ?15)",
                params![
                    source_key, candidate.canonical_url, candidate.external_id, candidate.source_domain,
                    candidate.title, candidate.company, candidate.description, candidate.location,
                    candidate.remote_status, candidate.employment_type, candidate.published_at,
                    candidate.expires_at, candidate.extraction_status, hash, timestamp,
                ],
            ).map_err(|error| error.to_string())?;
            (transaction.last_insert_rowid(), UpsertOutcome::New)
        };

        transaction
            .execute(
                "UPDATE discovery_results SET job_id=?2 WHERE id=?1",
                params![discovery_id, job_id],
            )
            .map_err(|error| error.to_string())?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO job_discoveries(job_id, discovery_id) VALUES(?1, ?2)",
                params![job_id, discovery_id],
            )
            .map_err(|error| error.to_string())?;
        transaction.execute(
            "INSERT INTO job_sources(job_id, provider, source_url, first_seen_at, last_seen_at) VALUES(?1, ?2, ?3, ?4, ?4)
             ON CONFLICT(job_id, provider, source_url) DO UPDATE SET last_seen_at=excluded.last_seen_at",
            params![job_id, provider, candidate.source_url, timestamp],
        ).map_err(|error| error.to_string())?;
        transaction.execute(
            "INSERT INTO extraction_attempts(discovery_id, status, strategy, attempted_at) VALUES(?1, 'success', ?2, ?3)",
            params![discovery_id, candidate.extraction_status, timestamp],
        ).map_err(|error| error.to_string())?;
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(outcome)
    }

    pub fn finish_run(&self, run_id: i64, status: &str, error: Option<&str>) -> Result<(), String> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE ingestion_runs SET completed_at=?2, status=?3, error_summary=?4,
             successful_queries=(SELECT COUNT(*) FROM query_executions WHERE run_id=?1 AND status='completed'),
             failed_queries=(SELECT COUNT(*) FROM query_executions WHERE run_id=?1 AND status='failed'),
             urls_discovered=(SELECT COUNT(*) FROM discovery_results WHERE run_id=?1),
             new_jobs=(SELECT COALESCE(SUM(new_jobs),0) FROM query_executions WHERE run_id=?1),
             updated_jobs=(SELECT COALESCE(SUM(updated_jobs),0) FROM query_executions WHERE run_id=?1),
             extraction_failures=(SELECT COUNT(*) FROM discovery_results WHERE run_id=?1 AND status='extraction_failed')
             WHERE id=?1",
            params![run_id, now(), status, error],
        ).map_err(|db_error| db_error.to_string())?;
        Ok(())
    }

    pub fn recover_interrupted_runs(&self) -> Result<(), String> {
        let connection = self.connect()?;
        connection.execute(
            "UPDATE ingestion_runs SET status='interrupted', completed_at=?1,
             error_summary='Rolebit closed before this ingestion completed.' WHERE status='running'",
            [now()],
        ).map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE query_executions SET status='interrupted', completed_at=?1,
             error='Rolebit closed before this query completed.' WHERE status='running'",
                [now()],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn overview(&self) -> Result<crate::models::Overview, String> {
        let connection = self.connect()?;
        let total = count(&connection, "SELECT COUNT(*) FROM jobs")?;
        let new_count = count(
            &connection,
            "SELECT COUNT(*) FROM jobs WHERE tracking_status='new'",
        )?;
        let updated = count(
            &connection,
            "SELECT COUNT(*) FROM jobs WHERE content_updated=1",
        )?;
        let active_queries =
            crate::query::generate_queries(&self.get_configuration()?).len() as i64;
        let latest_run = latest_run(&connection)?;
        let last_ingestion_at = latest_run.as_ref().and_then(|run| run.completed_at.clone());
        let ingestion_status = latest_run
            .as_ref()
            .map(|run| run.status.clone())
            .unwrap_or_else(|| "idle".into());
        let settings_interval = setting_i64(&connection, "schedule_interval_hours", 12)?;
        let schedule_enabled = setting_bool(&connection, "schedule_enabled", false)?;
        let next_ingestion_at = if schedule_enabled {
            last_ingestion_at
                .as_ref()
                .and_then(|value| chrono::DateTime::parse_from_rfc3339(value).ok())
                .map(|date| {
                    (date.with_timezone(&Utc) + Duration::hours(settings_interval)).to_rfc3339()
                })
        } else {
            None
        };
        let mut recent_statement = connection.prepare(
            "SELECT id,title,company,location,remote_status,source_domain,lifecycle_status,tracking_status,extraction_status,first_seen_at,last_seen_at,updated_at,content_updated
             FROM jobs ORDER BY first_seen_at DESC, id DESC LIMIT 5"
        ).map_err(|error| error.to_string())?;
        let recent_jobs = recent_statement
            .query_map([], row_to_job_summary)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        let mut activity_statement = connection.prepare(
            "SELECT substr(started_at,1,10), status, COALESCE(SUM(new_jobs),0) FROM ingestion_runs
             WHERE started_at >= datetime('now','-35 days') GROUP BY substr(started_at,1,10), status ORDER BY started_at"
        ).map_err(|error| error.to_string())?;
        let activity = activity_statement
            .query_map([], |row| {
                Ok(ActivityDay {
                    date: row.get(0)?,
                    status: row.get(1)?,
                    new_jobs: row.get(2)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(crate::models::Overview {
            total_opportunities: total,
            new_opportunities: new_count,
            recently_updated: updated,
            active_queries,
            last_ingestion_at,
            next_ingestion_at,
            ingestion_status,
            recent_jobs,
            activity,
            latest_run,
        })
    }

    pub fn list_jobs(&self, request: &JobListRequest) -> Result<JobPage, String> {
        let connection = self.connect()?;
        let page_size = match request.page_size {
            25 | 50 | 100 => request.page_size,
            _ => 25,
        };
        let page = request.page.max(1);
        let mut where_parts = Vec::new();
        let mut values: Vec<Value> = Vec::new();
        if let Some(search) = request
            .search
            .as_ref()
            .filter(|value| !value.trim().is_empty())
        {
            where_parts.push("id IN (SELECT rowid FROM jobs_fts WHERE jobs_fts MATCH ?)");
            values.push(Value::Text(format!(
                "\"{}\"",
                search.trim().replace('"', "\"\"")
            )));
        }
        push_filter(
            &mut where_parts,
            &mut values,
            "location",
            request.location.as_deref(),
        );
        push_filter(
            &mut where_parts,
            &mut values,
            "remote_status",
            request.remote_status.as_deref(),
        );
        push_filter(
            &mut where_parts,
            &mut values,
            "source_domain",
            request.source.as_deref(),
        );
        push_filter(
            &mut where_parts,
            &mut values,
            "lifecycle_status",
            request.lifecycle_status.as_deref(),
        );
        push_filter(
            &mut where_parts,
            &mut values,
            "tracking_status",
            request.tracking_status.as_deref(),
        );
        if let Some(date) = request
            .discovered_after
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            where_parts.push("first_seen_at >= ?");
            values.push(Value::Text(date.clone()));
        }
        let clause = if where_parts.is_empty() {
            String::new()
        } else {
            format!(" WHERE {}", where_parts.join(" AND "))
        };
        let count_sql = format!("SELECT COUNT(*) FROM jobs{clause}");
        let total: i64 = connection
            .query_row(&count_sql, params_from_iter(values.iter()), |row| {
                row.get(0)
            })
            .map_err(|error| error.to_string())?;
        let ordering = match request.sort.as_str() {
            "oldest" => "first_seen_at ASC, id ASC",
            "updated" => "updated_at DESC, id DESC",
            "seen" => "last_seen_at DESC, id DESC",
            "title" => "title COLLATE NOCASE ASC, id ASC",
            _ => "first_seen_at DESC, id DESC",
        };
        let list_sql = format!(
            "SELECT id,title,company,location,remote_status,source_domain,lifecycle_status,tracking_status,extraction_status,first_seen_at,last_seen_at,updated_at,content_updated
             FROM jobs{clause} ORDER BY {ordering} LIMIT ? OFFSET ?"
        );
        let mut list_values = values;
        list_values.push(Value::Integer(page_size as i64));
        list_values.push(Value::Integer(((page - 1) * page_size) as i64));
        let mut statement = connection
            .prepare(&list_sql)
            .map_err(|error| error.to_string())?;
        let items = statement
            .query_map(params_from_iter(list_values.iter()), row_to_job_summary)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        let locations = distinct_strings(
            &connection,
            "SELECT DISTINCT location FROM jobs WHERE location IS NOT NULL AND location <> '' ORDER BY location",
        )?;
        let sources = distinct_strings(
            &connection,
            "SELECT DISTINCT source_domain FROM jobs ORDER BY source_domain",
        )?;
        Ok(JobPage {
            items,
            total,
            page,
            page_size,
            facets: JobFacets { locations, sources },
        })
    }

    pub fn get_job(&self, id: i64) -> Result<JobDetail, String> {
        let connection = self.connect()?;
        let mut detail = connection.query_row(
            "SELECT id,title,company,location,remote_status,source_domain,lifecycle_status,tracking_status,extraction_status,first_seen_at,last_seen_at,updated_at,content_updated,
             description,canonical_url,external_id,employment_type,published_at,expires_at,reopened FROM jobs WHERE id=?1",
            [id],
            |row| Ok(JobDetail {
                summary: row_to_job_summary(row)?,
                description: row.get(13)?, canonical_url: row.get(14)?, external_id: row.get(15)?,
                employment_type: row.get(16)?, published_at: row.get(17)?, expires_at: row.get(18)?,
                reopened: row.get(19)?, discoveries: Vec::new(),
            }),
        ).map_err(|error| error.to_string())?;
        let mut statement = connection.prepare(
            "SELECT q.query,d.provider,d.discovered_url,d.discovered_at FROM job_discoveries jd
             JOIN discovery_results d ON d.id=jd.discovery_id
             JOIN query_executions qe ON qe.id=d.query_execution_id
             JOIN search_queries q ON q.id=qe.query_id WHERE jd.job_id=?1 ORDER BY d.discovered_at DESC"
        ).map_err(|error| error.to_string())?;
        detail.discoveries = statement
            .query_map([id], |row| {
                Ok(JobDiscovery {
                    query: row.get(0)?,
                    provider: row.get(1)?,
                    discovered_url: row.get(2)?,
                    discovered_at: row.get(3)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(detail)
    }

    pub fn update_tracking_status(&self, id: i64, status: &str) -> Result<(), String> {
        if !matches!(status, "new" | "seen" | "saved" | "dismissed") {
            return Err("Invalid tracking status".into());
        }
        let connection = self.connect()?;
        let changed = connection
            .execute(
                "UPDATE jobs SET tracking_status=?2 WHERE id=?1",
                params![id, status],
            )
            .map_err(|error| error.to_string())?;
        if changed == 0 {
            return Err("Opportunity not found".into());
        }
        Ok(())
    }

    pub fn list_runs(&self, limit: u32) -> Result<Vec<IngestionRun>, String> {
        let connection = self.connect()?;
        let mut statement = connection
            .prepare(&format!(
                "{} ORDER BY started_at DESC, id DESC LIMIT ?1",
                run_select()
            ))
            .map_err(|error| error.to_string())?;
        statement
            .query_map([limit.min(200)], row_to_run)
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn get_run(&self, id: i64) -> Result<IngestionRunDetail, String> {
        let connection = self.connect()?;
        let run = connection
            .query_row(&format!("{} WHERE id=?1", run_select()), [id], row_to_run)
            .map_err(|error| error.to_string())?;
        let mut statement = connection.prepare(
            "SELECT qe.id,q.query,qe.status,qe.result_count,qe.candidate_count,qe.new_jobs,qe.updated_jobs,qe.error,qe.started_at,qe.completed_at
             FROM query_executions qe JOIN search_queries q ON q.id=qe.query_id WHERE qe.run_id=?1 ORDER BY qe.id"
        ).map_err(|error| error.to_string())?;
        let queries = statement
            .query_map([id], |row| {
                Ok(QueryExecution {
                    id: row.get(0)?,
                    query: row.get(1)?,
                    status: row.get(2)?,
                    result_count: row.get(3)?,
                    candidate_count: row.get(4)?,
                    new_jobs: row.get(5)?,
                    updated_jobs: row.get(6)?,
                    error: row.get(7)?,
                    started_at: row.get(8)?,
                    completed_at: row.get(9)?,
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        Ok(IngestionRunDetail { run, queries })
    }

    pub fn diagnostics(&self, app_version: &str) -> Result<Diagnostics, String> {
        let connection = self.connect()?;
        let sqlite_version: String = connection
            .query_row("SELECT sqlite_version()", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        let schema_version: i64 = connection
            .query_row(
                "SELECT COALESCE(MAX(version),0) FROM schema_migrations",
                [],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        let database_size_bytes = fs::metadata(&self.path)
            .map(|metadata| metadata.len())
            .unwrap_or(0);
        Ok(Diagnostics {
            database_path: self.path.to_string_lossy().to_string(),
            database_size_bytes,
            sqlite_version,
            app_version: app_version.into(),
            schema_version,
        })
    }

    pub fn schedule_due(&self) -> Result<bool, String> {
        let connection = self.connect()?;
        if !setting_bool(&connection, "schedule_enabled", false)? {
            return Ok(false);
        }
        let interval = setting_i64(&connection, "schedule_interval_hours", 12)?;
        let last = setting_optional(&connection, "last_scheduled_run_at")?.or_else(|| {
            latest_run(&connection)
                .ok()
                .flatten()
                .map(|run| run.started_at)
        });
        Ok(
            match last.and_then(|value| chrono::DateTime::parse_from_rfc3339(&value).ok()) {
                Some(last) => {
                    Utc::now().signed_duration_since(last.with_timezone(&Utc))
                        >= Duration::hours(interval)
                }
                None => true,
            },
        )
    }
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn seed_defaults(transaction: &Transaction<'_>) -> Result<(), String> {
    let defaults = [
        ("provider", "mock"),
        ("serp_zone", ""),
        ("google_gl", "us"),
        ("google_hl", "en"),
        ("request_concurrency", "3"),
        ("request_timeout_seconds", "30"),
        ("retry_count", "2"),
        ("pages_per_query", "1"),
        ("estimated_cost_per_request", ""),
        ("request_budget", ""),
        ("schedule_enabled", "false"),
        ("schedule_interval_hours", "12"),
        ("run_on_startup", "false"),
        ("start_with_windows", "false"),
        ("close_to_tray", "true"),
        ("onboarding_complete", "false"),
    ];
    for (key, value) in defaults {
        transaction
            .execute(
                "INSERT OR IGNORE INTO application_settings(key,value,updated_at) VALUES(?1,?2,?3)",
                params![key, value, now()],
            )
            .map_err(|error| error.to_string())?;
    }
    let existing: i64 = transaction
        .query_row("SELECT COUNT(*) FROM search_items", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if existing == 0 {
        for (kind, values) in [
            (
                "site",
                vec![
                    "jobs.ashbyhq.com",
                    "jobs.lever.co",
                    "job-boards.greenhouse.io",
                ],
            ),
            ("location", vec!["Uruguay", "LATAM", "Remote", "Worldwide"]),
            (
                "role",
                vec!["QA", "SDET", "Software Engineer", "DevOps", "Product"],
            ),
        ] {
            for (index, value) in values.iter().enumerate() {
                transaction.execute(
                    "INSERT INTO search_items(kind,value,enabled,sort_order) VALUES(?1,?2,1,?3)",
                    params![kind, value, index as i64],
                ).map_err(|error| error.to_string())?;
            }
        }
    }
    Ok(())
}

fn insert_items(
    transaction: &Transaction<'_>,
    kind: &str,
    items: &[SearchItem],
) -> Result<(), String> {
    for (index, item) in items.iter().enumerate() {
        let value = item.value.trim();
        if value.is_empty() && kind == "site" {
            continue;
        }
        transaction.execute(
            "INSERT OR IGNORE INTO search_items(kind,value,enabled,sort_order) VALUES(?1,?2,?3,?4)",
            params![kind, value, item.enabled, index as i64],
        ).map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn set_setting(connection: &Connection, key: &str, value: &str) -> Result<(), String> {
    connection
        .execute(
            "INSERT INTO application_settings(key,value,updated_at) VALUES(?1,?2,?3)
         ON CONFLICT(key) DO UPDATE SET value=excluded.value,updated_at=excluded.updated_at",
            params![key, value, now()],
        )
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn setting(connection: &Connection, key: &str, default: &str) -> Result<String, String> {
    Ok(setting_optional(connection, key)?.unwrap_or_else(|| default.to_string()))
}

fn setting_optional(connection: &Connection, key: &str) -> Result<Option<String>, String> {
    connection
        .query_row(
            "SELECT value FROM application_settings WHERE key=?1",
            [key],
            |row| row.get(0),
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn setting_i64(connection: &Connection, key: &str, default: i64) -> Result<i64, String> {
    Ok(setting_optional(connection, key)?
        .and_then(|value| value.parse().ok())
        .unwrap_or(default))
}

fn setting_optional_i64(connection: &Connection, key: &str) -> Result<Option<i64>, String> {
    Ok(setting_optional(connection, key)?.and_then(|value| value.parse().ok()))
}

fn setting_optional_f64(connection: &Connection, key: &str) -> Result<Option<f64>, String> {
    Ok(setting_optional(connection, key)?.and_then(|value| value.parse().ok()))
}

fn setting_bool(connection: &Connection, key: &str, default: bool) -> Result<bool, String> {
    Ok(setting_optional(connection, key)?
        .and_then(|value| value.parse().ok())
        .unwrap_or(default))
}

fn candidate_hash(candidate: &CandidateJob) -> String {
    let meaningful = serde_json::json!({
        "title": candidate.title.trim(), "company": candidate.company, "description": candidate.description,
        "location": candidate.location, "remote": candidate.remote_status, "employment": candidate.employment_type,
        "published": candidate.published_at, "expires": candidate.expires_at,
    });
    hex::encode(Sha256::digest(meaningful.to_string().as_bytes()))
}

fn row_to_job_summary(row: &Row<'_>) -> rusqlite::Result<JobSummary> {
    Ok(JobSummary {
        id: row.get(0)?,
        title: row.get(1)?,
        company: row.get(2)?,
        location: row.get(3)?,
        remote_status: row.get(4)?,
        source_domain: row.get(5)?,
        lifecycle_status: row.get(6)?,
        tracking_status: row.get(7)?,
        extraction_status: row.get(8)?,
        first_seen_at: row.get(9)?,
        last_seen_at: row.get(10)?,
        updated_at: row.get(11)?,
        content_updated: row.get(12)?,
    })
}

fn run_select() -> &'static str {
    "SELECT id,started_at,completed_at,trigger_type,status,total_queries,attempted_requests,successful_queries,failed_queries,urls_discovered,new_jobs,updated_jobs,extraction_failures,estimated_cost,error_summary FROM ingestion_runs"
}

fn row_to_run(row: &Row<'_>) -> rusqlite::Result<IngestionRun> {
    Ok(IngestionRun {
        id: row.get(0)?,
        started_at: row.get(1)?,
        completed_at: row.get(2)?,
        trigger_type: row.get(3)?,
        status: row.get(4)?,
        total_queries: row.get(5)?,
        attempted_requests: row.get(6)?,
        successful_queries: row.get(7)?,
        failed_queries: row.get(8)?,
        urls_discovered: row.get(9)?,
        new_jobs: row.get(10)?,
        updated_jobs: row.get(11)?,
        extraction_failures: row.get(12)?,
        estimated_cost: row.get(13)?,
        error_summary: row.get(14)?,
    })
}

fn latest_run(connection: &Connection) -> Result<Option<IngestionRun>, String> {
    connection
        .query_row(
            &format!("{} ORDER BY started_at DESC, id DESC LIMIT 1", run_select()),
            [],
            row_to_run,
        )
        .optional()
        .map_err(|error| error.to_string())
}

fn count(connection: &Connection, sql: &str) -> Result<i64, String> {
    connection
        .query_row(sql, [], |row| row.get(0))
        .map_err(|error| error.to_string())
}

fn push_filter(
    parts: &mut Vec<&'static str>,
    values: &mut Vec<Value>,
    column: &str,
    value: Option<&str>,
) {
    if let Some(value) = value.filter(|value| !value.is_empty() && *value != "all") {
        let expression = match column {
            "location" => "location = ?",
            "remote_status" => "remote_status = ?",
            "source_domain" => "source_domain = ?",
            "lifecycle_status" => "lifecycle_status = ?",
            "tracking_status" => "tracking_status = ?",
            _ => return,
        };
        parts.push(expression);
        values.push(Value::Text(value.to_string()));
    }
}

fn distinct_strings(connection: &Connection, sql: &str) -> Result<Vec<String>, String> {
    let mut statement = connection.prepare(sql).map_err(|error| error.to_string())?;
    statement
        .query_map([], |row| row.get(0))
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use std::{fs, sync::Arc, thread};

    use super::*;

    fn test_database(name: &str) -> (Database, PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "rolebit-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let database = Database::initialize(directory.join("rolebit.sqlite3")).unwrap();
        (database, directory)
    }

    fn candidate(description: &str) -> CandidateJob {
        CandidateJob {
            source_url: "https://jobs.example.com/roles/quality-42?utm_source=search".into(),
            canonical_url: "https://jobs.example.com/roles/quality-42".into(),
            external_id: Some("quality-42".into()),
            source_domain: "jobs.example.com".into(),
            title: "Senior Quality Engineer".into(),
            company: Some("Example Labs".into()),
            description: Some(description.into()),
            location: Some("Remote".into()),
            remote_status: "remote".into(),
            employment_type: Some("Full-time".into()),
            published_at: Some("2026-10-01".into()),
            expires_at: None,
            extraction_status: "json_ld".into(),
        }
    }

    fn request() -> JobListRequest {
        JobListRequest {
            search: None,
            location: None,
            remote_status: None,
            source: None,
            lifecycle_status: None,
            tracking_status: None,
            discovered_after: None,
            sort: "newest".into(),
            page: 1,
            page_size: 25,
        }
    }

    #[test]
    fn migrates_and_persists_default_configuration() {
        let (database, directory) = test_database("migrations");
        let configuration = database.get_configuration().unwrap();
        assert_eq!(configuration.sites.len(), 3);
        assert_eq!(crate::query::generate_queries(&configuration).len(), 60);
        assert_eq!(database.diagnostics("test").unwrap().schema_version, 1);
        drop(database);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn upserts_revisions_without_overwriting_tracking_state() {
        let (database, directory) = test_database("upsert");
        let run_id = database.start_run("test", 2, None).unwrap();
        let first_execution = database
            .start_query_execution(run_id, "first query")
            .unwrap();
        let first = database
            .record_candidate(
                run_id,
                first_execution,
                "mock",
                None,
                None,
                &candidate("Original description"),
            )
            .unwrap();
        assert_eq!(first, UpsertOutcome::New);
        let page = database.list_jobs(&request()).unwrap();
        let job_id = page.items[0].id;
        database.update_tracking_status(job_id, "saved").unwrap();

        let second_execution = database
            .start_query_execution(run_id, "second query")
            .unwrap();
        let updated = database
            .record_candidate(
                run_id,
                second_execution,
                "mock",
                None,
                None,
                &candidate("Meaningfully changed description"),
            )
            .unwrap();
        assert_eq!(updated, UpsertOutcome::Updated);
        let detail = database.get_job(job_id).unwrap();
        assert_eq!(detail.summary.tracking_status, "saved");
        assert!(detail.summary.content_updated);
        assert_eq!(detail.discoveries.len(), 2);
        let discovery_queries = detail
            .discoveries
            .iter()
            .map(|discovery| discovery.query.as_str())
            .collect::<Vec<_>>();
        assert!(discovery_queries.contains(&"first query"));
        assert!(discovery_queries.contains(&"second query"));

        let unchanged = database
            .record_candidate(
                run_id,
                second_execution,
                "mock",
                None,
                None,
                &candidate("Meaningfully changed description"),
            )
            .unwrap();
        assert_eq!(unchanged, UpsertOutcome::Existing);
        assert_eq!(database.list_jobs(&request()).unwrap().total, 1);
        drop(database);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn searches_filters_and_paginates_in_sqlite() {
        let (database, directory) = test_database("listing");
        let run_id = database.start_run("test", 1, None).unwrap();
        let execution = database
            .start_query_execution(run_id, "quality query")
            .unwrap();
        database
            .record_candidate(
                run_id,
                execution,
                "mock",
                None,
                None,
                &candidate("Build excellent testing systems"),
            )
            .unwrap();
        let mut filtered = request();
        filtered.search = Some("testing systems".into());
        filtered.remote_status = Some("remote".into());
        assert_eq!(database.list_jobs(&filtered).unwrap().total, 1);
        filtered.location = Some("Nowhere".into());
        assert_eq!(database.list_jobs(&filtered).unwrap().total, 0);
        drop(database);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn serializes_concurrent_discovery_into_one_job() {
        let (database, directory) = test_database("concurrent");
        let run_id = database.start_run("test", 2, None).unwrap();
        let first_execution = database
            .start_query_execution(run_id, "remote quality")
            .unwrap();
        let second_execution = database
            .start_query_execution(run_id, "latam quality")
            .unwrap();
        let database = Arc::new(database);
        let handles = [first_execution, second_execution].map(|execution| {
            let database = database.clone();
            thread::spawn(move || {
                database.record_candidate(
                    run_id,
                    execution,
                    "mock",
                    None,
                    None,
                    &candidate("Concurrent description"),
                )
            })
        });
        let outcomes = handles.map(|handle| handle.join().unwrap().unwrap());
        assert!(outcomes.contains(&UpsertOutcome::New));
        assert!(outcomes.contains(&UpsertOutcome::Existing));
        assert_eq!(database.list_jobs(&request()).unwrap().total, 1);
        drop(database);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn schedules_one_missed_run_and_waits_for_the_next_interval() {
        let (database, directory) = test_database("schedule");
        {
            let connection = database.connect().unwrap();
            set_setting(&connection, "schedule_enabled", "true").unwrap();
            set_setting(&connection, "schedule_interval_hours", "12").unwrap();
        }
        assert!(database.schedule_due().unwrap());
        database.set_last_scheduled_run().unwrap();
        assert!(!database.schedule_due().unwrap());
        drop(database);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn recovers_interrupted_runs_and_query_executions() {
        let (database, directory) = test_database("recovery");
        let run_id = database.start_run("scheduled", 1, None).unwrap();
        database
            .start_query_execution(run_id, "site:example.com remote qa")
            .unwrap();
        database.recover_interrupted_runs().unwrap();
        let run = database.list_runs(10).unwrap().remove(0);
        assert_eq!(run.status, "interrupted");
        assert!(run.completed_at.is_some());
        drop(database);
        fs::remove_dir_all(directory).unwrap();
    }
}
