use tauri::{AppHandle, State};
use tauri_plugin_autostart::ManagerExt;
use serde_json::json;
use std::time::Duration;

use crate::{
    ingestion::{AppState, try_start_ingestion},
    models::{
        Bootstrap, CompleteOnboardingInput, Diagnostics, IngestionRun, IngestionRunDetail,
        JobDetail, JobListRequest, JobPage, Profile, QueryPreview, QueryPreviewInput,
        SaveSettingsInput, SearchConfiguration,
    },
    provider::BrightDataProvider,
    query, secret,
};

#[tauri::command]
pub fn get_bootstrap(state: State<'_, AppState>) -> Result<Bootstrap, String> {
    let settings = get_settings_inner(&state)?;
    Ok(Bootstrap {
        configuration: state.db.get_configuration()?,
        overview: state.db.overview()?,
        recent_runs: state.db.list_runs(12)?,
        settings,
    })
}

#[tauri::command]
pub fn get_search_configuration(state: State<'_, AppState>) -> Result<SearchConfiguration, String> {
    state.db.get_configuration()
}

#[tauri::command]
pub fn save_search_configuration(
    state: State<'_, AppState>,
    configuration: SearchConfiguration,
) -> Result<SearchConfiguration, String> {
    state.db.save_configuration(&configuration)?;
    state.db.get_configuration()
}

#[tauri::command]
pub fn get_profile(state: State<'_, AppState>) -> Result<Profile, String> {
    state.db.get_profile()
}

#[tauri::command]
pub fn save_profile(state: State<'_, AppState>, profile: Profile) -> Result<Profile, String> {
    state.db.save_profile(&profile)
}

#[tauri::command]
pub fn preview_queries(
    state: State<'_, AppState>,
    input: QueryPreviewInput,
) -> Result<QueryPreview, String> {
    let settings = get_settings_inner(&state)?;
    Ok(query::preview(
        &input.configuration,
        input
            .schedule_interval_hours
            .unwrap_or(settings.schedule_interval_hours),
        input
            .estimated_cost_per_request
            .unwrap_or(settings.estimated_cost_per_request),
    ))
}

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Result<crate::models::AppSettings, String> {
    get_settings_inner(&state)
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    state: State<'_, AppState>,
    settings: SaveSettingsInput,
) -> Result<crate::models::AppSettings, String> {
    save_settings_inner(&app, &state, &settings)?;
    get_settings_inner(&state)
}

#[tauri::command]
pub fn complete_onboarding(
    app: AppHandle,
    state: State<'_, AppState>,
    input: CompleteOnboardingInput,
) -> Result<Bootstrap, String> {
    save_settings_inner(&app, &state, &input.settings)?;
    state.db.save_configuration(&input.configuration)?;
    state.db.mark_onboarding_complete()?;
    get_bootstrap(state)
}

#[tauri::command]
pub async fn test_bright_data_connection(
    state: State<'_, AppState>,
    api_key: Option<String>,
    zone: String,
) -> Result<String, String> {
    let mut settings = get_settings_inner(&state)?;
    settings.serp_zone = zone;
    let key = api_key
        .filter(|value| !value.trim().is_empty())
        .or(secret::get_api_key()?)
        .ok_or_else(|| "Enter a Bright Data API key first".to_string())?;
    BrightDataProvider::new(&settings, key)?
        .test_connection()
        .await
}

#[tauri::command]
pub async fn test_decisions_connection(
    api_key: Option<String>,
) -> Result<String, String> {
    let key = api_key
        .filter(|value| !value.trim().is_empty())
        .or(secret::get_decisions_api_key()?)
        .ok_or_else(|| "Enter an OpenAI Decisions API key first".to_string())?;
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .user_agent("Rolebit/0.1")
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .post("https://api.openai.com/v1/decisions")
        .bearer_auth(key.trim())
        .json(&json!({
            "model": "gpt-6-luna",
            "input": "Connection test.",
            "questions": [{
                "type": "predicate",
                "name": "connection",
                "instructions": "Is this connection test input available?"
            }]
        }))
        .send()
        .await
        .map_err(|error| format!("Decisions API request failed: {error}"))?;
    let status = response.status();
    let body = response.text().await.map_err(|error| error.to_string())?;
    if !status.is_success() {
        return Err(format!("Decisions API returned {status}: {}", response_preview(&body)));
    }
    let value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|error| format!("Decisions API returned invalid JSON: {error}"))?;
    if value.get("answers").and_then(serde_json::Value::as_array).is_none() {
        return Err("Decisions API returned a response without answers".into());
    }
    Ok("Connection succeeded. Decisions API accepted a test decision.".into())
}

#[tauri::command]
pub fn start_ingestion(app: AppHandle, state: State<'_, AppState>) -> Result<i64, String> {
    try_start_ingestion(app, state.inner().clone(), "manual")
}

#[tauri::command]
pub fn cancel_ingestion(state: State<'_, AppState>) -> Result<(), String> {
    if !state.is_running() {
        return Err("No ingestion run is active".into());
    }
    state.cancel();
    Ok(())
}

#[tauri::command]
pub fn get_overview(state: State<'_, AppState>) -> Result<crate::models::Overview, String> {
    state.db.overview()
}

#[tauri::command]
pub fn list_jobs(state: State<'_, AppState>, request: JobListRequest) -> Result<JobPage, String> {
    state.db.list_jobs(&request)
}

#[tauri::command]
pub fn get_job(state: State<'_, AppState>, id: i64) -> Result<JobDetail, String> {
    state.db.get_job(id)
}

#[tauri::command]
pub fn update_tracking_status(
    state: State<'_, AppState>,
    id: i64,
    status: String,
) -> Result<JobDetail, String> {
    state.db.update_tracking_status(id, &status)?;
    state.db.get_job(id)
}

#[tauri::command]
pub fn list_ingestion_runs(
    state: State<'_, AppState>,
    limit: Option<u32>,
) -> Result<Vec<IngestionRun>, String> {
    state.db.list_runs(limit.unwrap_or(100))
}

#[tauri::command]
pub fn get_ingestion_run(
    state: State<'_, AppState>,
    id: i64,
) -> Result<IngestionRunDetail, String> {
    state.db.get_run(id)
}

#[tauri::command]
pub fn get_diagnostics(state: State<'_, AppState>) -> Result<Diagnostics, String> {
    state.db.diagnostics(env!("CARGO_PKG_VERSION"))
}

fn get_settings_inner(state: &AppState) -> Result<crate::models::AppSettings, String> {
    state
        .db
        .get_settings(
            env!("CARGO_PKG_VERSION"),
            secret::get_api_key()?.is_some(),
            secret::get_decisions_api_key()?.is_some(),
        )
}

fn save_settings_inner(
    app: &AppHandle,
    state: &AppState,
    settings: &SaveSettingsInput,
) -> Result<(), String> {
    if let Some(api_key) = settings.api_key.as_ref() {
        if api_key.trim().is_empty() {
            secret::delete_api_key()?;
        } else {
            secret::set_api_key(api_key.trim())?;
        }
    }
    if let Some(api_key) = settings.decisions_api_key.as_ref() {
        if api_key.trim().is_empty() {
            secret::delete_decisions_api_key()?;
        } else {
            secret::set_decisions_api_key(api_key.trim())?;
        }
    }
    state.db.save_settings(settings)?;
    if settings.start_with_windows {
        app.autolaunch()
            .enable()
            .map_err(|error| error.to_string())?;
    } else {
        app.autolaunch()
            .disable()
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn response_preview(body: &str) -> String {
    let compact = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > 240 {
        format!("{}…", compact.chars().take(240).collect::<String>())
    } else {
        compact
    }
}
