mod ats;
mod commands;
mod decision;
mod db;
mod extractor;
mod ingestion;
mod models;
mod provider;
mod query;
mod secret;

use std::time::Duration;

use ingestion::{AppState, try_start_ingestion};
use tauri::{
    Manager, WindowEvent,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .arg("--from-autostart")
                .app_name("Rolebit")
                .build(),
        )
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let app_data_dir = app
                .path()
                .app_data_dir()
                .map_err(|error| error.to_string())?;
            let database = db::Database::initialize(app_data_dir.join("rolebit.sqlite3"))?;
            database.recover_interrupted_runs()?;
            let state = AppState::new(database);
            app.manage(state.clone());
            build_tray(app)?;
            start_scheduler(app.handle().clone(), state);
            if std::env::args().any(|arg| arg == "--from-autostart")
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.hide();
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                let close_to_tray = window
                    .state::<AppState>()
                    .db
                    .get_settings(env!("CARGO_PKG_VERSION"), false, false)
                    .map(|settings| settings.close_to_tray)
                    .unwrap_or(true);
                if close_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::get_search_configuration,
            commands::save_search_configuration,
            commands::get_profile,
            commands::save_profile,
            commands::preview_queries,
            commands::get_settings,
            commands::save_settings,
            commands::complete_onboarding,
            commands::test_bright_data_connection,
            commands::test_decisions_connection,
            commands::start_ingestion,
            commands::cancel_ingestion,
            commands::get_overview,
            commands::list_jobs,
            commands::get_job,
            commands::update_tracking_status,
            commands::list_ingestion_runs,
            commands::get_ingestion_run,
            commands::get_diagnostics,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Rolebit");
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open = MenuItem::with_id(app, "open", "Open Rolebit", true, None::<&str>)?;
    let run = MenuItem::with_id(app, "run", "Run ingestion", true, None::<&str>)?;
    let schedule = MenuItem::with_id(
        app,
        "schedule",
        "Pause / resume scheduling",
        true,
        None::<&str>,
    )?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &run, &schedule, &quit])?;
    let mut builder = TrayIconBuilder::new().menu(&menu).tooltip("Rolebit");
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, event| match event.id().as_ref() {
            "open" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
            "run" => {
                let state = app.state::<AppState>().inner().clone();
                let _ = try_start_ingestion(app.clone(), state, "tray");
            }
            "schedule" => {
                let state = app.state::<AppState>();
                if let Ok(settings) = state
                    .db
                    .get_settings(env!("CARGO_PKG_VERSION"), false, false)
                {
                    let _ = state.db.set_schedule_enabled(!settings.schedule_enabled);
                }
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .build(app)?;
    Ok(())
}

fn start_scheduler(app: tauri::AppHandle, state: AppState) {
    tauri::async_runtime::spawn(async move {
        let mut first_check = true;
        loop {
            tokio::time::sleep(if first_check {
                Duration::from_secs(5)
            } else {
                Duration::from_secs(60)
            })
            .await;
            let has_key = secret::get_api_key().ok().flatten().is_some();
            if let Ok(settings) = state
                .db
                .get_settings(env!("CARGO_PKG_VERSION"), has_key, false)
            {
                let should_run_startup = first_check && settings.run_on_startup;
                let should_run_scheduled = state.db.schedule_due().unwrap_or(false);
                if (should_run_startup || should_run_scheduled) && !state.is_running() {
                    let trigger = if should_run_startup {
                        "startup"
                    } else {
                        "scheduled"
                    };
                    let _ = try_start_ingestion(app.clone(), state.clone(), trigger);
                }
            }
            first_check = false;
        }
    });
}
