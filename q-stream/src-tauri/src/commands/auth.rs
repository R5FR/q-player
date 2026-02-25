use crate::models::SessionInfo;
use crate::qobuz::QobuzClient;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;
use tracing::{info, warn};

// ── Session file helpers ──

fn session_file_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("q-stream")
        .join("session.json")
}

fn save_session_to_disk(client: &QobuzClient) {
    let path = session_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match serde_json::to_string_pretty(&client.to_saved()) {
        Ok(json) => {
            if std::fs::write(&path, json).is_ok() {
                info!("Session saved to {}", path.display());
            }
        }
        Err(e) => warn!("Failed to serialize session: {}", e),
    }
}

fn clear_session_from_disk() {
    let path = session_file_path();
    if path.exists() {
        let _ = std::fs::remove_file(&path);
        info!("Session file cleared");
    }
}

// ── Commands ──

/// Restore the previous session from disk without requiring a re-login.
/// Returns a logged-out SessionInfo if no saved session exists.
#[tauri::command]
pub async fn restore_session(
    state: State<'_, Arc<AppState>>,
) -> Result<SessionInfo, String> {
    let path = session_file_path();
    if !path.exists() {
        return Ok(SessionInfo { logged_in: false, user_name: None, subscription: None });
    }

    let json = match std::fs::read_to_string(&path) {
        Ok(j) => j,
        Err(e) => {
            warn!("Failed to read session file: {}", e);
            return Ok(SessionInfo { logged_in: false, user_name: None, subscription: None });
        }
    };

    let saved: crate::models::SavedSession = match serde_json::from_str(&json) {
        Ok(s) => s,
        Err(e) => {
            warn!("Session file invalid, removing: {}", e);
            let _ = std::fs::remove_file(&path);
            return Ok(SessionInfo { logged_in: false, user_name: None, subscription: None });
        }
    };

    info!("Restoring session for user: {}", saved.user_name);
    let client = QobuzClient::from_saved(&saved);
    let session_info = client.session_info();
    *state.qobuz.write() = Some(client);
    Ok(session_info)
}

#[tauri::command]
pub async fn login(
    email: String,
    password: String,
    state: State<'_, Arc<AppState>>,
) -> Result<SessionInfo, String> {
    let client = QobuzClient::login(&email, &password)
        .await
        .map_err(|e| format!("Login failed: {}", e))?;

    let session = client.session_info();
    save_session_to_disk(&client);
    *state.qobuz.write() = Some(client);

    Ok(session)
}

#[tauri::command]
pub async fn logout(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    *state.qobuz.write() = None;
    clear_session_from_disk();
    Ok(())
}

#[tauri::command]
pub async fn get_session(state: State<'_, Arc<AppState>>) -> Result<SessionInfo, String> {
    let qobuz = state.qobuz.read();
    match qobuz.as_ref() {
        Some(client) => Ok(client.session_info()),
        None => Ok(SessionInfo {
            logged_in: false,
            user_name: None,
            subscription: None,
        }),
    }
}
