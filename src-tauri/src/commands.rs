use crate::db::{NewReply, NewThread};
use crate::error::{AppError, AppResult};
use crate::models::{Draft, RefreshReport, Settings, ThreadDetail, ThreadSummary};
use crate::{lore, mail, secrets, AppState};
use tauri::{AppHandle, Emitter, State};

pub const THREADS_UPDATED: &str = "threads-updated";

/// Keyring access talks to the Secret Service over D-Bus and can block on an
/// unlock prompt, so it never runs on the UI thread.
async fn keyring<T: Send + 'static>(f: impl FnOnce() -> AppResult<T> + Send + 'static) -> AppResult<T> {
    tauri::async_runtime::spawn_blocking(f)
        .await
        .map_err(|e| AppError::Other(format!("keyring task failed: {e}")))?
}

#[tauri::command]
pub async fn get_settings(state: State<'_, AppState>) -> AppResult<Settings> {
    let mut s = state.db.lock().unwrap().load_settings()?;
    s.has_password = keyring(secrets::get_password).await?.is_some();
    Ok(s)
}

#[tauri::command]
pub async fn save_settings(
    state: State<'_, AppState>,
    settings: Settings,
    password: Option<String>,
) -> AppResult<Settings> {
    // Keyring first: if it fails nothing else is half-saved.
    if let Some(p) = password.filter(|p| !p.is_empty()) {
        if settings.smtp_user.is_empty() {
            return Err(AppError::Settings("enter the SMTP username before saving a password".into()));
        }
        let p = secrets::normalize(&p);
        keyring(move || secrets::set_password(&p)).await?;
    }
    state.db.lock().unwrap().save_settings(&settings)?;
    state.poll_changed.notify_one();
    let mut s = settings;
    s.has_password = keyring(secrets::get_password).await?.is_some();
    Ok(s)
}

#[tauri::command]
pub async fn forget_password(state: State<'_, AppState>) -> AppResult<Settings> {
    keyring(secrets::delete_password).await?;
    let mut s = state.db.lock().unwrap().load_settings()?;
    s.has_password = false;
    Ok(s)
}

/// The password to use for `settings`: a typed one wins, otherwise the keyring.
async fn resolve_password(settings: &Settings, typed: Option<String>) -> AppResult<String> {
    match typed.filter(|p| !p.is_empty()) {
        Some(p) => Ok(secrets::normalize(&p)),
        None if settings.smtp_user.is_empty() => Ok(String::new()),
        None => keyring(secrets::get_password)
            .await?
            .ok_or_else(|| AppError::Settings("no SMTP password saved in Settings".into())),
    }
}

/// Try the SMTP settings as given, without saving anything. Returns a
/// sentence saying what was verified.
#[tauri::command]
pub async fn test_smtp(settings: Settings, password: Option<String>) -> AppResult<String> {
    let password = resolve_password(&settings, password).await?;
    mail::test_connection(&settings, &password).await
}

#[tauri::command]
pub fn default_recipient() -> String {
    mail::LKML.to_string()
}

#[tauri::command]
pub async fn send_email(
    app: AppHandle,
    state: State<'_, AppState>,
    draft: Draft,
) -> AppResult<ThreadSummary> {
    let settings = state.db.lock().unwrap().load_settings()?;
    let password = resolve_password(&settings, None).await?;
    let message_id = mail::new_message_id(&settings.email);
    let email = mail::build(&settings, &draft, &message_id)?;
    // Record the thread before the irreversible send. SMTP cannot tell us
    // whether a failure happened before or after the server queued the
    // message, so on error the row is kept and marked unconfirmed rather
    // than deleted: lore is the arbiter of whether it went out.
    let now = chrono::Utc::now().to_rfc3339();
    let lore_url = mail::lore_url(&message_id);
    let id = state.db.lock().unwrap().insert_thread(&NewThread {
        message_id: &message_id,
        subject: draft.subject.trim(),
        to_addr: &draft.to,
        cc: &draft.cc,
        body: &draft.body,
        sent_at: &now,
        lore_url: &lore_url,
    })?;
    let result = mail::send(&settings, &password, email).await;
    if let Err(e) = result {
        state.db.lock().unwrap().set_status(id, "unconfirmed")?;
        let _ = app.emit(THREADS_UPDATED, ());
        return Err(AppError::Other(format!(
            "{e}. The draft is kept as an unconfirmed thread; if it shows up on lore it was delivered, otherwise stop tracking it and send again."
        )));
    }
    let _ = app.emit(THREADS_UPDATED, ());
    let detail = state.db.lock().unwrap().get_thread(id)?;
    detail
        .map(|d| d.summary)
        .ok_or_else(|| AppError::Other("thread vanished after insert".into()))
}

#[tauri::command]
pub fn list_threads(state: State<AppState>) -> AppResult<Vec<ThreadSummary>> {
    state.db.lock().unwrap().list_threads()
}

#[tauri::command]
pub fn get_thread(state: State<AppState>, id: i64) -> AppResult<Option<ThreadDetail>> {
    state.db.lock().unwrap().get_thread(id)
}

#[tauri::command]
pub fn mark_thread_read(app: AppHandle, state: State<AppState>, id: i64) -> AppResult<()> {
    state.db.lock().unwrap().mark_thread_read(id)?;
    let _ = app.emit(THREADS_UPDATED, ());
    Ok(())
}

#[tauri::command]
pub fn delete_thread(app: AppHandle, state: State<AppState>, id: i64) -> AppResult<()> {
    state.db.lock().unwrap().delete_thread(id)?;
    let _ = app.emit(THREADS_UPDATED, ());
    Ok(())
}

#[tauri::command]
pub async fn refresh_thread(
    app: AppHandle,
    state: State<'_, AppState>,
    id: i64,
) -> AppResult<RefreshReport> {
    let report = refresh_one(&state, id).await?;
    let _ = app.emit(THREADS_UPDATED, ());
    Ok(report)
}

#[tauri::command]
pub async fn refresh_all(
    app: AppHandle,
    state: State<'_, AppState>,
) -> AppResult<Vec<RefreshReport>> {
    let reports = refresh_every_thread(&state).await?;
    let _ = app.emit(THREADS_UPDATED, ());
    Ok(reports)
}

pub async fn refresh_every_thread(state: &AppState) -> AppResult<Vec<RefreshReport>> {
    let ids = state.db.lock().unwrap().thread_ids()?;
    let mut reports = Vec::with_capacity(ids.len());
    for id in ids {
        reports.push(match refresh_one(state, id).await {
            Ok(r) => r,
            Err(e) => RefreshReport { thread_id: id, new_replies: 0, on_lore: false, error: Some(e.to_string()) },
        });
    }
    Ok(reports)
}

async fn refresh_one(state: &AppState, id: i64) -> AppResult<RefreshReport> {
    let message_id = state
        .db
        .lock()
        .unwrap()
        .thread_message_id(id)?
        .ok_or_else(|| AppError::Other(format!("no thread with id {id}")))?;
    let now = chrono::Utc::now().to_rfc3339();
    let messages = match lore::fetch_thread(&state.http, &message_id).await {
        Ok(m) => m,
        Err(AppError::NotOnLore) => {
            state.db.lock().unwrap().touch_checked(id, &now)?;
            return Ok(RefreshReport { thread_id: id, new_replies: 0, on_lore: false, error: None });
        }
        Err(e) => return Err(e),
    };
    let root = mail::bare_id(&message_id);
    let replies: Vec<NewReply> = messages
        .iter()
        .filter(|m| mail::bare_id(&m.message_id) != root)
        .map(|m| NewReply {
            thread_id: id,
            message_id: &m.message_id,
            in_reply_to: m.in_reply_to.as_deref(),
            from_name: &m.from_name,
            from_addr: &m.from_addr,
            date: &m.date,
            subject: &m.subject,
            body: &m.body,
            lore_url: mail::lore_url(&m.message_id),
        })
        .collect();
    let db = state.db.lock().unwrap();
    let new_replies = db.insert_replies(&replies)?;
    // Seeing the thread on lore proves the message was delivered.
    db.set_status(id, "sent")?;
    db.touch_checked(id, &now)?;
    Ok(RefreshReport { thread_id: id, new_replies, on_lore: true, error: None })
}
