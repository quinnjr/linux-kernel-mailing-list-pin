use crate::db::{NewReply, NewThread};
use crate::error::{AppError, AppResult};
use crate::models::{Draft, RefreshReport, Settings, ThreadDetail, ThreadSummary};
use crate::{lore, mail, secrets, AppState};
use tauri::{AppHandle, Emitter, State};

pub const THREADS_UPDATED: &str = "threads-updated";

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> AppResult<Settings> {
    let mut s = state.db.lock().unwrap().load_settings()?;
    s.has_password = secrets::has_password(&s.smtp_user);
    Ok(s)
}

#[tauri::command]
pub fn save_settings(
    state: State<AppState>,
    settings: Settings,
    password: Option<String>,
) -> AppResult<Settings> {
    // Keyring first: if it fails nothing else is half-saved.
    if let Some(p) = password.filter(|p| !p.is_empty()) {
        if settings.smtp_user.is_empty() {
            return Err(AppError::Settings("enter the SMTP username before saving a password".into()));
        }
        secrets::set_password(&settings.smtp_user, &p)?;
    }
    state.db.lock().unwrap().save_settings(&settings)?;
    state.poll_changed.notify_one();
    let mut s = settings;
    s.has_password = secrets::has_password(&s.smtp_user);
    Ok(s)
}

/// Try the SMTP settings as given. A non-empty `password` is used as-is
/// (without being saved); otherwise the keyring entry for `smtp_user` is used.
#[tauri::command]
pub async fn test_smtp(settings: Settings, password: Option<String>) -> AppResult<()> {
    let password = match password.filter(|p| !p.is_empty()) {
        Some(p) => p,
        None if settings.smtp_user.is_empty() => String::new(),
        None => secrets::get_password(&settings.smtp_user)?
            .ok_or_else(|| AppError::Settings("no password entered or saved".into()))?,
    };
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
    let password = if settings.smtp_user.is_empty() {
        String::new()
    } else {
        secrets::get_password(&settings.smtp_user)?
            .ok_or_else(|| AppError::Settings("no SMTP password saved in Settings".into()))?
    };
    let message_id = mail::new_message_id(&settings.email);
    let email = mail::build(&settings, &draft, &message_id)?;
    // Record the thread before the irreversible send, so a database failure
    // can never leave a delivered message untracked; roll back if SMTP fails.
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
    if let Err(e) = mail::send(&settings, &password, email).await {
        let _ = state.db.lock().unwrap().delete_thread(id);
        return Err(e);
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
    let db = state.db.lock().unwrap();
    let mut new_replies = 0;
    let root = mail::bare_id(&message_id);
    for m in messages.iter().filter(|m| mail::bare_id(&m.message_id) != root) {
        let inserted = db.insert_reply(&NewReply {
            thread_id: id,
            message_id: &m.message_id,
            in_reply_to: m.in_reply_to.as_deref(),
            from_name: &m.from_name,
            from_addr: &m.from_addr,
            date: &m.date,
            subject: &m.subject,
            body: &m.body,
            lore_url: &mail::lore_url(&m.message_id),
        })?;
        if inserted {
            new_replies += 1;
        }
    }
    db.touch_checked(id, &now)?;
    Ok(RefreshReport { thread_id: id, new_replies, on_lore: true, error: None })
}
