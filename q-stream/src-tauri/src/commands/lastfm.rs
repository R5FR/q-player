use crate::lastfm::LastFmClient;
use crate::models::LastFmUserSession;
use crate::state::AppState;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::State;
use tracing::warn;

// ── Persistence helpers ──

fn lastfm_file_path() -> std::path::PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("q-stream")
        .join("lastfm.json")
}

fn save_lastfm_to_disk(session: &LastFmUserSession) {
    let path = lastfm_file_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(session) {
        let _ = std::fs::write(&path, json);
    }
}

fn load_lastfm_from_disk() -> Option<LastFmUserSession> {
    let json = std::fs::read_to_string(lastfm_file_path()).ok()?;
    serde_json::from_str(&json).ok()
}

// ── Auth commands ──

/// Step 1 of Last.fm OAuth: generate a token and return the URL the user
/// must visit to grant Q-Stream access to their Last.fm account.
#[tauri::command]
pub async fn lastfm_start_auth(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let client = LastFmClient::new();
    let token = client.get_auth_token().await?;
    let url = client.auth_url(&token);
    *state.lastfm_pending_token.write() = Some(token);
    Ok(url)
}

/// Step 2 of Last.fm OAuth: exchange the pending token for a permanent session key.
/// The user must have already authorized the app at the URL from `lastfm_start_auth`.
#[tauri::command]
pub async fn lastfm_complete_auth(
    state: State<'_, Arc<AppState>>,
) -> Result<LastFmUserSession, String> {
    let token = state
        .lastfm_pending_token
        .read()
        .clone()
        .ok_or("No pending Last.fm auth. Call lastfm_start_auth first.")?;

    let client = LastFmClient::new();
    let session = client.get_session(&token).await?;

    save_lastfm_to_disk(&session);
    *state.lastfm.write() = Some(session.clone());
    *state.lastfm_pending_token.write() = None;

    Ok(session)
}

/// Returns the active Last.fm session (restores from disk on first call if needed).
#[tauri::command]
pub async fn lastfm_get_session(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<LastFmUserSession>, String> {
    // Already loaded in memory?
    {
        let guard = state.lastfm.read();
        if guard.is_some() {
            return Ok(guard.clone());
        }
    }
    // Try disk
    if let Some(saved) = load_lastfm_from_disk() {
        *state.lastfm.write() = Some(saved.clone());
        return Ok(Some(saved));
    }
    Ok(None)
}

/// Disconnect from Last.fm and remove stored credentials.
#[tauri::command]
pub async fn lastfm_disconnect(
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    *state.lastfm.write() = None;
    let _ = std::fs::remove_file(lastfm_file_path());
    Ok(())
}

// ── Scrobbling commands ──

/// Notify Last.fm that a track started playing (fires when track is loaded).
#[tauri::command]
pub async fn lastfm_now_playing(
    track: String,
    artist: String,
    duration_secs: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let key = {
        let guard = state.lastfm.read();
        guard.as_ref().map(|s| s.session_key.clone())
    };
    let Some(key) = key else {
        return Ok(()); // silently skip if not connected
    };

    LastFmClient::new()
        .update_now_playing(&key, &track, &artist, duration_secs)
        .await
        .unwrap_or_else(|e| warn!("now_playing failed: {}", e));

    Ok(())
}

/// Scrobble a track (fire after ≥50% listened or ≥4 minutes).
#[tauri::command]
pub async fn lastfm_scrobble(
    track: String,
    artist: String,
    duration_secs: u32,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let key = {
        let guard = state.lastfm.read();
        guard.as_ref().map(|s| s.session_key.clone())
    };
    let Some(key) = key else {
        return Ok(());
    };

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    LastFmClient::new()
        .scrobble(&key, &track, &artist, timestamp, duration_secs)
        .await
        .unwrap_or_else(|e| warn!("scrobble failed: {}", e));

    Ok(())
}
