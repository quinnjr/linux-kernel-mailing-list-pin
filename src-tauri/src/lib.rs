mod commands;
mod db;
mod error;
mod lore;
mod mail;
mod models;
mod secrets;

use std::sync::Mutex;
use std::time::Duration;
use tauri::{Emitter, Manager};
use tokio::sync::Notify;

pub struct AppState {
    pub db: Mutex<db::Db>,
    pub http: reqwest::Client,
    /// Signalled when settings change so the poller can pick up a new interval.
    pub poll_changed: Notify,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = db::Db::open(&dir.join("lkml-pin.sqlite"))?;
            let http = reqwest::Client::builder()
                .user_agent(concat!("lkml-pin/", env!("CARGO_PKG_VERSION")))
                .timeout(Duration::from_secs(30))
                .build()?;
            app.manage(AppState { db: Mutex::new(db), http, poll_changed: Notify::new() });

            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move { poll_loop(handle).await });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::default_recipient,
            commands::send_email,
            commands::list_threads,
            commands::get_thread,
            commands::mark_thread_read,
            commands::delete_thread,
            commands::refresh_thread,
            commands::refresh_all,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

async fn poll_loop(app: tauri::AppHandle) {
    loop {
        let state = app.state::<AppState>();
        let minutes = state
            .db
            .lock()
            .unwrap()
            .load_settings()
            .map(|s| s.poll_minutes)
            .unwrap_or(15)
            .max(1);
        let sleep = tokio::time::sleep(Duration::from_secs(u64::from(minutes) * 60));
        tokio::select! {
            _ = sleep => {
                let reports = commands::refresh_every_thread(&state).await;
                let fresh: usize = reports.iter().map(|r| r.new_replies).sum();
                let _ = app.emit(commands::THREADS_UPDATED, ());
                if fresh > 0 {
                    let _ = app.emit("new-replies", fresh);
                }
            }
            _ = state.poll_changed.notified() => {}
        }
    }
}
