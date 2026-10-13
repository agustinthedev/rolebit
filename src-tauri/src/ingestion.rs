use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use futures::{StreamExt, stream};
use tauri::{AppHandle, Emitter};
use tokio::sync::OnceCell;
use url::Url;

use crate::{
    db::{Database, UpsertOutcome},
    extractor::{fetch_and_extract, is_non_job_result, mock_candidate},
    models::{AppSettings, IngestionProgress},
    provider::{BrightDataProvider, SearchResponse, mock_results},
    query::generate_queries,
    secret,
};

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    running: Arc<AtomicBool>,
    cancel: Arc<AtomicBool>,
}

impl AppState {
    pub fn new(db: Database) -> Self {
        Self {
            db,
            running: Arc::new(AtomicBool::new(false)),
            cancel: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    fn try_acquire_run(&self) -> Result<(), String> {
        self.running
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map(|_| ())
            .map_err(|_| "An ingestion run is already in progress".into())
    }

    fn release_run(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

#[derive(Clone)]
enum Provider {
    Mock,
    Bright(Arc<BrightDataProvider>),
}

impl Provider {
    async fn search(&self, query: &str, page: u32) -> Result<SearchResponse, String> {
        match self {
            Self::Mock => Ok(SearchResponse {
                results: mock_results(query, page),
                diagnostic: None,
            }),
            Self::Bright(provider) => provider.search(query, page).await,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Self::Mock => "mock",
            Self::Bright(_) => "bright_data",
        }
    }
}

struct Counters {
    completed: AtomicUsize,
    urls: AtomicUsize,
    new_jobs: AtomicUsize,
    updated_jobs: AtomicUsize,
    errors: AtomicUsize,
    query_failures: AtomicUsize,
    extraction_failures: AtomicUsize,
    filtered: AtomicUsize,
    successes: AtomicUsize,
}

type ExtractionCache =
    Arc<Mutex<HashMap<String, Arc<OnceCell<Result<crate::models::CandidateJob, String>>>>>>;

pub fn try_start_ingestion(app: AppHandle, state: AppState, trigger: &str) -> Result<i64, String> {
    state.try_acquire_run()?;
    let configuration = match state.db.get_configuration() {
        Ok(value) => value,
        Err(error) => {
            state.release_run();
            return Err(error);
        }
    };
    let settings = match current_settings(&state) {
        Ok(value) => value,
        Err(error) => {
            state.release_run();
            return Err(error);
        }
    };
    let queries = generate_queries(&configuration);
    if queries.is_empty() {
        state.release_run();
        return Err("No valid enabled search combinations are configured".into());
    }
    let planned_requests = queries.len() * configuration.pages_per_query.max(1) as usize;
    if let Some(budget) = settings.request_budget
        && planned_requests > budget as usize
    {
        state.release_run();
        return Err(format!(
            "This run plans {planned_requests} requests, above the configured budget of {budget}"
        ));
    }
    let estimated_cost = settings
        .estimated_cost_per_request
        .map(|cost| cost * planned_requests as f64);
    let run_id = match state.db.start_run(trigger, queries.len(), estimated_cost) {
        Ok(id) => id,
        Err(error) => {
            state.release_run();
            return Err(error);
        }
    };
    state.cancel.store(false, Ordering::SeqCst);
    if trigger == "scheduled" {
        let _ = state.db.set_last_scheduled_run();
    }
    tauri::async_runtime::spawn(async move {
        execute(
            app,
            state,
            run_id,
            settings,
            queries,
            configuration.pages_per_query.max(1),
        )
        .await;
    });
    Ok(run_id)
}

async fn execute(
    app: AppHandle,
    state: AppState,
    run_id: i64,
    settings: AppSettings,
    queries: Vec<String>,
    pages: u32,
) {
    let provider = match build_provider(&settings) {
        Ok(provider) => provider,
        Err(error) => {
            let _ = state.db.finish_run(run_id, "failed", Some(&error));
            state.running.store(false, Ordering::SeqCst);
            let _ = app.emit(
                "ingestion-progress",
                IngestionProgress {
                    run_id,
                    status: "failed".into(),
                    current_query: None,
                    completed_queries: 0,
                    total_queries: queries.len(),
                    urls_discovered: 0,
                    new_jobs: 0,
                    updated_jobs: 0,
                    errors: 1,
                    filtered_results: 0,
                },
            );
            return;
        }
    };
    let counters = Arc::new(Counters {
        completed: AtomicUsize::new(0),
        urls: AtomicUsize::new(0),
        new_jobs: AtomicUsize::new(0),
        updated_jobs: AtomicUsize::new(0),
        errors: AtomicUsize::new(0),
        query_failures: AtomicUsize::new(0),
        extraction_failures: AtomicUsize::new(0),
        filtered: AtomicUsize::new(0),
        successes: AtomicUsize::new(0),
    });
    let extraction_cache: ExtractionCache = Arc::new(Mutex::new(HashMap::new()));
    let total = queries.len();
    emit_progress(&app, run_id, "running", None, total, &counters);

    stream::iter(queries)
        .for_each_concurrent(settings.request_concurrency.max(1) as usize, |query| {
            let app = app.clone();
            let state = state.clone();
            let settings = settings.clone();
            let provider = provider.clone();
            let counters = counters.clone();
            let extraction_cache = extraction_cache.clone();
            async move {
                if state.cancel.load(Ordering::SeqCst) {
                    return;
                }
                emit_progress(
                    &app,
                    run_id,
                    "running",
                    Some(query.clone()),
                    total,
                    &counters,
                );
                match process_query(
                    &state,
                    run_id,
                    &settings,
                    &provider,
                    &query,
                    pages,
                    &extraction_cache,
                    &counters,
                )
                .await
                {
                    Ok(()) => {
                        counters.successes.fetch_add(1, Ordering::SeqCst);
                    }
                    Err(_) => {
                        counters.errors.fetch_add(1, Ordering::SeqCst);
                        counters.query_failures.fetch_add(1, Ordering::SeqCst);
                    }
                }
                counters.completed.fetch_add(1, Ordering::SeqCst);
                emit_progress(&app, run_id, "running", Some(query), total, &counters);
            }
        })
        .await;

    let cancelled = state.cancel.load(Ordering::SeqCst);
    let errors = counters.errors.load(Ordering::SeqCst);
    let query_failures = counters.query_failures.load(Ordering::SeqCst);
    let extraction_failures = counters.extraction_failures.load(Ordering::SeqCst);
    let filtered = counters.filtered.load(Ordering::SeqCst);
    let successes = counters.successes.load(Ordering::SeqCst);
    let status = if cancelled {
        "cancelled"
    } else if errors > 0 && successes > 0 {
        "partial"
    } else if errors > 0 {
        "failed"
    } else {
        "completed"
    };
    let mut summary = Vec::new();
    if query_failures > 0 {
        summary.push(format!(
            "{query_failures} search request{} failed",
            if query_failures == 1 { "" } else { "s" }
        ));
    }
    if extraction_failures > 0 {
        summary.push(format!(
            "{extraction_failures} candidate extraction{} failed",
            if extraction_failures == 1 { "" } else { "s" }
        ));
    }
    if filtered > 0 {
        summary.push(format!(
            "{filtered} non-matching or generic result{} filtered",
            if filtered == 1 { "" } else { "s" }
        ));
    }
    let error = (!summary.is_empty()).then(|| summary.join("; "));
    let _ = state.db.finish_run(run_id, status, error.as_deref());
    state.release_run();
    emit_progress(&app, run_id, status, None, total, &counters);
}

#[allow(clippy::too_many_arguments)]
async fn process_query(
    state: &AppState,
    run_id: i64,
    settings: &AppSettings,
    provider: &Provider,
    query: &str,
    pages: u32,
    extraction_cache: &ExtractionCache,
    counters: &Arc<Counters>,
) -> Result<(), String> {
    let execution_id = state.db.start_query_execution(run_id, query)?;
    let mut result_count = 0;
    let mut candidate_count = 0;
    let mut filtered_count = 0;
    let mut new_jobs = 0;
    let mut updated_jobs = 0;
    let mut diagnostic = None;
    let expected_host = expected_site_host(query);
    for page in 1..=pages {
        if state.cancel.load(Ordering::SeqCst) {
            break;
        }
        state.db.record_request_attempt(run_id)?;
        let response = match provider.search(query, page).await {
            Ok(response) => response,
            Err(error) => {
                state.db.finish_query_execution(
                    execution_id,
                    "failed",
                    result_count,
                    candidate_count,
                    filtered_count,
                    new_jobs,
                    updated_jobs,
                    Some(&error),
                )?;
                return Err(error);
            }
        };
        if diagnostic.is_none() {
            diagnostic = response.diagnostic;
        }
        let results = response.results;
        result_count += results.len();
        for result in results {
            if state.cancel.load(Ordering::SeqCst) {
                break;
            }
            counters.urls.fetch_add(1, Ordering::SeqCst);
            if let Some(expected_host) = expected_host.as_deref()
                && !url_matches_host(expected_host, &result.url)
            {
                let error = format!(
                    "Filtered result outside expected site domain {expected_host}: {}",
                    result.url
                );
                state.db.record_filtered_discovery(
                    run_id,
                    execution_id,
                    provider.name(),
                    &result.url,
                    result.title.as_deref(),
                    result.snippet.as_deref(),
                    &error,
                )?;
                filtered_count += 1;
                counters.filtered.fetch_add(1, Ordering::SeqCst);
                continue;
            }
            candidate_count += 1;
            let extracted = extract_candidate(
                extraction_cache,
                provider,
                query,
                &result.url,
                settings.request_timeout_seconds,
            )
            .await;
            match extracted {
                Ok(candidate) => {
                    if let Some(expected_host) = expected_host.as_deref()
                        && !url_matches_host(expected_host, &candidate.canonical_url)
                    {
                        let error = format!(
                            "Filtered canonical URL outside expected site domain {expected_host}: {}",
                            candidate.canonical_url
                        );
                        state.db.record_filtered_discovery(
                            run_id,
                            execution_id,
                            provider.name(),
                            &result.url,
                            result.title.as_deref(),
                            result.snippet.as_deref(),
                            &error,
                        )?;
                        filtered_count += 1;
                        counters.filtered.fetch_add(1, Ordering::SeqCst);
                        continue;
                    }
                    match state.db.record_candidate(
                        run_id,
                        execution_id,
                        provider.name(),
                        result.title.as_deref(),
                        result.snippet.as_deref(),
                        &candidate,
                    )? {
                        UpsertOutcome::New => {
                            new_jobs += 1;
                            counters.new_jobs.fetch_add(1, Ordering::SeqCst);
                        }
                        UpsertOutcome::Updated => {
                            updated_jobs += 1;
                            counters.updated_jobs.fetch_add(1, Ordering::SeqCst);
                        }
                        UpsertOutcome::Existing => {}
                    }
                }
                Err(error) if is_non_job_result(&error) => {
                    state.db.record_filtered_discovery(
                        run_id,
                        execution_id,
                        provider.name(),
                        &result.url,
                        result.title.as_deref(),
                        result.snippet.as_deref(),
                        &error,
                    )?;
                    filtered_count += 1;
                    counters.filtered.fetch_add(1, Ordering::SeqCst);
                }
                Err(error) => {
                    state.db.record_failed_discovery(
                        run_id,
                        execution_id,
                        provider.name(),
                        &result.url,
                        result.title.as_deref(),
                        result.snippet.as_deref(),
                        &error,
                    )?;
                    counters.errors.fetch_add(1, Ordering::SeqCst);
                    counters.extraction_failures.fetch_add(1, Ordering::SeqCst);
                }
            }
        }
    }
    let status = if state.cancel.load(Ordering::SeqCst) {
        "cancelled"
    } else {
        "completed"
    };
    state.db.finish_query_execution(
        execution_id,
        status,
        result_count,
        candidate_count,
        filtered_count,
        new_jobs,
        updated_jobs,
        diagnostic.as_deref(),
    )
}

fn expected_site_host(query: &str) -> Option<String> {
    query.split_whitespace().find_map(|part| {
        let value = part.strip_prefix("site:")?.trim_matches(['"', '\'']);
        let value = value.trim_end_matches('/');
        let url = Url::parse(&format!("https://{value}")).ok()?;
        url.host_str()
            .map(|host| host.trim_end_matches('.').to_ascii_lowercase())
    })
}

fn url_matches_host(expected_host: &str, value: &str) -> bool {
    Url::parse(value)
        .ok()
        .and_then(|url| url.host_str().map(str::to_ascii_lowercase))
        .is_some_and(|host| host == expected_host || host.ends_with(&format!(".{expected_host}")))
}

async fn extract_candidate(
    extraction_cache: &ExtractionCache,
    provider: &Provider,
    query: &str,
    url: &str,
    timeout_seconds: u32,
) -> Result<crate::models::CandidateJob, String> {
    let cell = {
        let mut cache = extraction_cache
            .lock()
            .map_err(|_| "Extraction cache unavailable".to_string())?;
        cache
            .entry(url.to_string())
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone()
    };
    let extracted = cell
        .get_or_init(|| async {
            match provider {
                Provider::Mock => Ok(mock_candidate(query, url)),
                Provider::Bright(_) => fetch_and_extract(url, timeout_seconds).await,
            }
        })
        .await
        .clone();
    if extracted.is_err() {
        let mut cache = extraction_cache
            .lock()
            .map_err(|_| "Extraction cache unavailable".to_string())?;
        if cache
            .get(url)
            .is_some_and(|current| Arc::ptr_eq(current, &cell))
        {
            cache.remove(url);
        }
    }
    extracted
}

fn current_settings(state: &AppState) -> Result<AppSettings, String> {
    let has_key = secret::get_api_key()?.is_some();
    state.db.get_settings(env!("CARGO_PKG_VERSION"), has_key)
}

fn build_provider(settings: &AppSettings) -> Result<Provider, String> {
    match settings.provider.as_str() {
        "mock" => Ok(Provider::Mock),
        "bright_data" => {
            let key = secret::get_api_key()?
                .ok_or_else(|| "Bright Data API key is not configured".to_string())?;
            Ok(Provider::Bright(Arc::new(BrightDataProvider::new(
                settings, key,
            )?)))
        }
        _ => Err("Unsupported search provider".into()),
    }
}

fn emit_progress(
    app: &AppHandle,
    run_id: i64,
    status: &str,
    current_query: Option<String>,
    total: usize,
    counters: &Counters,
) {
    let _ = app.emit(
        "ingestion-progress",
        IngestionProgress {
            run_id,
            status: status.into(),
            current_query,
            completed_queries: counters.completed.load(Ordering::SeqCst),
            total_queries: total,
            urls_discovered: counters.urls.load(Ordering::SeqCst),
            new_jobs: counters.new_jobs.load(Ordering::SeqCst),
            updated_jobs: counters.updated_jobs.load(Ordering::SeqCst),
            errors: counters.errors.load(Ordering::SeqCst),
            filtered_results: counters.filtered.load(Ordering::SeqCst),
        },
    );
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn state(name: &str) -> (AppState, std::path::PathBuf) {
        let directory = std::env::temp_dir().join(format!(
            "rolebit-ingestion-{name}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&directory).unwrap();
        let database = Database::initialize(directory.join("rolebit.sqlite3")).unwrap();
        (AppState::new(database), directory)
    }

    #[test]
    fn prevents_overlapping_runs_and_releases_the_guard() {
        let (state, directory) = state("overlap");
        assert!(state.try_acquire_run().is_ok());
        assert!(state.try_acquire_run().is_err());
        state.release_run();
        assert!(state.try_acquire_run().is_ok());
        drop(state);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn cancellation_is_shared_across_state_clones() {
        let (state, directory) = state("cancel");
        let worker = state.clone();
        state.cancel();
        assert!(worker.cancel.load(Ordering::SeqCst));
        drop(worker);
        drop(state);
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn matches_site_domains_without_accepting_lookalikes() {
        assert_eq!(
            expected_site_host("site:jobs.example.com \"Remote\" \"QA\""),
            Some("jobs.example.com".into())
        );
        assert!(url_matches_host(
            "jobs.example.com",
            "https://careers.jobs.example.com/role/42"
        ));
        assert!(!url_matches_host(
            "jobs.example.com",
            "https://jobs.example.com.evil.test/role/42"
        ));
        assert!(!url_matches_host(
            "jobs.example.com",
            "https://www.linkedin.com/jobs/view/42"
        ));
    }

    #[tokio::test]
    async fn caches_successful_extraction_but_keeps_each_query_attribution_path() {
        let cache: ExtractionCache = Arc::new(Mutex::new(HashMap::new()));
        let provider = Provider::Mock;
        let first = extract_candidate(
            &cache,
            &provider,
            "site:jobs.example.com \"Remote\" \"QA\"",
            "https://jobs.example.com/42",
            30,
        )
        .await
        .unwrap();
        let second = extract_candidate(
            &cache,
            &provider,
            "site:jobs.example.com \"LATAM\" \"SDET\"",
            "https://jobs.example.com/42",
            30,
        )
        .await
        .unwrap();
        assert_eq!(first.title, second.title);
        assert_eq!(cache.lock().unwrap().len(), 1);
    }
}
