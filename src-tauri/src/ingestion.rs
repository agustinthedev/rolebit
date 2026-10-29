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

use crate::{
    db::{Database, StoredDecision, UpsertOutcome},
    decision::{DecisionsClient, PageDecision, decision_input_hash, matching_profile_hash},
    extractor::{ExtractedPage, fetch_and_extract, mock_candidate},
    models::{AppSettings, IngestionProgress, Profile},
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

type ExtractionCache = Arc<Mutex<HashMap<String, Arc<OnceCell<Result<ExtractedPage, String>>>>>>;
type DecisionCache = Arc<Mutex<HashMap<String, Arc<OnceCell<Result<PageDecision, String>>>>>>;

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
    if settings.provider == "bright_data" && !settings.has_decisions_api_key {
        state.release_run();
        return Err("OpenAI Decisions API key is not configured".into());
    }
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
    let decisions_client = match &provider {
        Provider::Mock => None,
        Provider::Bright(_) => match secret::get_decisions_api_key().and_then(|key| {
            let key =
                key.ok_or_else(|| "OpenAI Decisions API key is not configured".to_string())?;
            DecisionsClient::new(key, settings.request_timeout_seconds)
        }) {
            Ok(client) => Some(client),
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
        },
    };
    let profile = match state.db.get_profile() {
        Ok(profile) => profile,
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
    let decision_cache: DecisionCache = Arc::new(Mutex::new(HashMap::new()));
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
            let decision_cache = decision_cache.clone();
            let decisions_client = decisions_client.clone();
            let profile = profile.clone();
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
                    &decision_cache,
                    decisions_client.as_ref(),
                    &profile,
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
    decision_cache: &DecisionCache,
    decisions_client: Option<&DecisionsClient>,
    profile: &Profile,
    counters: &Arc<Counters>,
) -> Result<(), String> {
    let execution_id = state.db.start_query_execution(run_id, query)?;
    let mut result_count = 0;
    let mut candidate_count = 0;
    let mut filtered_count = 0;
    let mut new_jobs = 0;
    let mut updated_jobs = 0;
    let mut diagnostic = None;
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
            candidate_count += 1;
            let extracted = extract_candidate(
                extraction_cache,
                provider,
                query,
                &result.url,
                settings.request_timeout_seconds,
                result.title.as_deref(),
                result.snippet.as_deref(),
            )
            .await;
            match extracted {
                Ok(page) => {
                    let candidate = &page.candidate;
                    let stored_decision = state.db.stored_decision(candidate)?;
                    match decide_page(
                        decision_cache,
                        decisions_client,
                        &page,
                        profile,
                        stored_decision,
                    )
                    .await
                    {
                        Ok(decision) if decision.is_job_posting() => {
                            match state.db.record_candidate_with_decision_cache(
                                run_id,
                                execution_id,
                                provider.name(),
                                result.title.as_deref(),
                                result.snippet.as_deref(),
                                candidate,
                                decision.confidence,
                                decision.matching.as_ref(),
                                decision.matching_error.as_deref(),
                                decision.decision_input_hash.as_deref(),
                                decision.match_profile_hash.as_deref(),
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
                        Ok(decision) => {
                            let confidence = decision
                                .confidence
                                .map(|value| format!(" ({:.0}% confidence)", value * 100.0))
                                .unwrap_or_default();
                            let reason = format!(
                                "Decisions API classified this page as {}{}",
                                decision.category.replace('_', " "),
                                confidence
                            );
                            state.db.record_filtered_discovery(
                                run_id,
                                execution_id,
                                provider.name(),
                                &result.url,
                                result.title.as_deref(),
                                result.snippet.as_deref(),
                                &reason,
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

async fn extract_candidate(
    extraction_cache: &ExtractionCache,
    provider: &Provider,
    query: &str,
    url: &str,
    timeout_seconds: u32,
    result_title: Option<&str>,
    result_snippet: Option<&str>,
) -> Result<ExtractedPage, String> {
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
                Provider::Mock => {
                    let candidate = mock_candidate(query, url);
                    let page_text = format!(
                        "{} {}",
                        candidate.title,
                        candidate.description.as_deref().unwrap_or_default()
                    );
                    Ok(ExtractedPage {
                        candidate,
                        page_text,
                    })
                }
                Provider::Bright(_) => {
                    fetch_and_extract(url, timeout_seconds, result_title, result_snippet).await
                }
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

async fn decide_page(
    cache: &DecisionCache,
    client: Option<&DecisionsClient>,
    page: &ExtractedPage,
    profile: &Profile,
    stored: Option<StoredDecision>,
) -> Result<PageDecision, String> {
    let Some(client) = client else {
        return Ok(PageDecision::local_job_posting());
    };
    let url = page.candidate.canonical_url.clone();
    let input_hash = decision_input_hash(&page.candidate, &page.page_text);
    let profile_hash = matching_profile_hash(profile);
    let content_unchanged = stored
        .as_ref()
        .is_some_and(|stored| stored.decision_input_hash.as_deref() == Some(input_hash.as_str()));
    let cell = {
        let mut cache = cache
            .lock()
            .map_err(|_| "Decision cache unavailable".to_string())?;
        cache
            .entry(url.clone())
            .or_insert_with(|| Arc::new(OnceCell::new()))
            .clone()
    };
    let decision = cell
        .get_or_init(|| async {
            let mut decision = if content_unchanged {
                let stored = stored.as_ref().expect("unchanged decision exists");
                PageDecision {
                    category: "job_posting".into(),
                    confidence: stored.classification_confidence,
                    matching: stored.matching.clone(),
                    matching_error: stored.match_error.clone(),
                    decision_input_hash: Some(input_hash.clone()),
                    match_profile_hash: stored.match_profile_hash.clone(),
                }
            } else {
                client
                    .classify_page(&page.candidate, &page.page_text)
                    .await?
            };
            decision.decision_input_hash = Some(input_hash.clone());
            if decision.is_job_posting() {
                let match_unchanged = content_unchanged
                    && stored.as_ref().is_some_and(|stored| {
                        stored.match_profile_hash.as_deref() == Some(profile_hash.as_str())
                    });
                if match_unchanged {
                    return Ok(decision);
                }
                decision.match_profile_hash = Some(profile_hash.clone());
                match client
                    .score_job(&page.candidate, &page.page_text, profile)
                    .await
                {
                    Ok(matching) => decision.matching = matching,
                    Err(error) => decision.matching_error = Some(error),
                }
            }
            Ok(decision)
        })
        .await
        .clone();
    if decision.is_err() {
        let mut cache = cache
            .lock()
            .map_err(|_| "Decision cache unavailable".to_string())?;
        if cache
            .get(&url)
            .is_some_and(|current| Arc::ptr_eq(current, &cell))
        {
            cache.remove(&url);
        }
    }
    decision
}

fn current_settings(state: &AppState) -> Result<AppSettings, String> {
    let has_key = secret::get_api_key()?.is_some();
    let has_decisions_api_key = secret::get_decisions_api_key()?.is_some();
    state
        .db
        .get_settings(env!("CARGO_PKG_VERSION"), has_key, has_decisions_api_key)
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
            None,
            None,
        )
        .await
        .unwrap();
        let second = extract_candidate(
            &cache,
            &provider,
            "site:jobs.example.com \"LATAM\" \"SDET\"",
            "https://jobs.example.com/42",
            30,
            None,
            None,
        )
        .await
        .unwrap();
        assert_eq!(first.candidate.title, second.candidate.title);
        assert_eq!(cache.lock().unwrap().len(), 1);
    }
}
