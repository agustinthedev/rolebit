use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SearchItem {
    pub id: Option<i64>,
    pub value: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchConfiguration {
    pub sites: Vec<SearchItem>,
    pub locations: Vec<SearchItem>,
    pub roles: Vec<SearchItem>,
    pub pages_per_query: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPreview {
    pub queries: Vec<String>,
    pub enabled_sites: usize,
    pub enabled_locations: usize,
    pub enabled_roles: usize,
    pub total_requests: usize,
    pub estimated_monthly_requests: usize,
    pub estimated_cost: Option<f64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryPreviewInput {
    pub configuration: SearchConfiguration,
    #[serde(default)]
    pub schedule_interval_hours: Option<u32>,
    #[serde(default)]
    pub estimated_cost_per_request: Option<Option<f64>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub onboarding_complete: bool,
    pub provider: String,
    pub serp_zone: String,
    pub google_gl: String,
    pub google_hl: String,
    pub request_concurrency: u32,
    pub request_timeout_seconds: u32,
    pub retry_count: u32,
    pub pages_per_query: u32,
    pub estimated_cost_per_request: Option<f64>,
    pub request_budget: Option<u32>,
    pub schedule_enabled: bool,
    pub schedule_interval_hours: u32,
    pub run_on_startup: bool,
    pub start_with_windows: bool,
    pub close_to_tray: bool,
    pub last_scheduled_run_at: Option<String>,
    pub has_api_key: bool,
    pub database_path: String,
    pub app_version: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveSettingsInput {
    pub provider: String,
    pub api_key: Option<String>,
    pub serp_zone: String,
    pub google_gl: String,
    pub google_hl: String,
    pub request_concurrency: u32,
    pub request_timeout_seconds: u32,
    pub retry_count: u32,
    pub pages_per_query: u32,
    pub estimated_cost_per_request: Option<f64>,
    pub request_budget: Option<u32>,
    pub schedule_enabled: bool,
    pub schedule_interval_hours: u32,
    pub run_on_startup: bool,
    pub start_with_windows: bool,
    pub close_to_tray: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompleteOnboardingInput {
    pub settings: SaveSettingsInput,
    pub configuration: SearchConfiguration,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    pub total_opportunities: i64,
    pub new_opportunities: i64,
    pub recently_updated: i64,
    pub active_queries: i64,
    pub last_ingestion_at: Option<String>,
    pub next_ingestion_at: Option<String>,
    pub ingestion_status: String,
    pub recent_jobs: Vec<JobSummary>,
    pub activity: Vec<ActivityDay>,
    pub latest_run: Option<IngestionRun>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivityDay {
    pub date: String,
    pub status: String,
    pub new_jobs: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobSummary {
    pub id: i64,
    pub title: String,
    pub company: Option<String>,
    pub location: Option<String>,
    pub remote_status: String,
    pub source_domain: String,
    pub lifecycle_status: String,
    pub tracking_status: String,
    pub extraction_status: String,
    pub first_seen_at: String,
    pub last_seen_at: String,
    pub updated_at: String,
    pub content_updated: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDetail {
    #[serde(flatten)]
    pub summary: JobSummary,
    pub description: Option<String>,
    pub canonical_url: String,
    pub external_id: Option<String>,
    pub employment_type: Option<String>,
    pub published_at: Option<String>,
    pub expires_at: Option<String>,
    pub reopened: bool,
    pub discoveries: Vec<JobDiscovery>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobDiscovery {
    pub query: String,
    pub provider: String,
    pub discovered_url: String,
    pub discovered_at: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobListRequest {
    pub search: Option<String>,
    pub location: Option<String>,
    pub remote_status: Option<String>,
    pub source: Option<String>,
    pub lifecycle_status: Option<String>,
    pub tracking_status: Option<String>,
    pub discovered_after: Option<String>,
    pub sort: String,
    pub page: u32,
    pub page_size: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobPage {
    pub items: Vec<JobSummary>,
    pub total: i64,
    pub page: u32,
    pub page_size: u32,
    pub facets: JobFacets,
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobFacets {
    pub locations: Vec<String>,
    pub sources: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionRun {
    pub id: i64,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub trigger_type: String,
    pub status: String,
    pub total_queries: i64,
    pub attempted_requests: i64,
    pub successful_queries: i64,
    pub failed_queries: i64,
    pub urls_discovered: i64,
    pub new_jobs: i64,
    pub updated_jobs: i64,
    pub extraction_failures: i64,
    pub estimated_cost: Option<f64>,
    pub error_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QueryExecution {
    pub id: i64,
    pub query: String,
    pub status: String,
    pub result_count: i64,
    pub candidate_count: i64,
    pub new_jobs: i64,
    pub updated_jobs: i64,
    pub error: Option<String>,
    pub started_at: String,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionRunDetail {
    #[serde(flatten)]
    pub run: IngestionRun,
    pub queries: Vec<QueryExecution>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestionProgress {
    pub run_id: i64,
    pub status: String,
    pub current_query: Option<String>,
    pub completed_queries: usize,
    pub total_queries: usize,
    pub urls_discovered: usize,
    pub new_jobs: usize,
    pub updated_jobs: usize,
    pub errors: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct CandidateJob {
    pub source_url: String,
    pub canonical_url: String,
    pub external_id: Option<String>,
    pub source_domain: String,
    pub title: String,
    pub company: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub remote_status: String,
    pub employment_type: Option<String>,
    pub published_at: Option<String>,
    pub expires_at: Option<String>,
    pub extraction_status: String,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub url: String,
    pub title: Option<String>,
    pub snippet: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub database_path: String,
    pub database_size_bytes: u64,
    pub sqlite_version: String,
    pub app_version: String,
    pub schema_version: i64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    pub settings: AppSettings,
    pub configuration: SearchConfiguration,
    pub overview: Overview,
    pub recent_runs: Vec<IngestionRun>,
}
