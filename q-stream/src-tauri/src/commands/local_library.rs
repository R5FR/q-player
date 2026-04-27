use crate::config;
use crate::local_library;
use crate::models::*;
use crate::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn import_folder(
    folder_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<LocalTrack>, String> {
    let path = PathBuf::from(&folder_path);
    if !path.exists() || !path.is_dir() {
        return Err(format!("Directory not found: {}", folder_path));
    }

    let tracks = tokio::task::spawn_blocking(move || local_library::scan_directory(&path))
        .await
        .map_err(|e| format!("Scan failed: {}", e))?;

    // Merge into state
    let mut local = state.local_tracks.write();
    for track in &tracks {
        // Avoid duplicates
        if !local.iter().any(|t| t.file_path == track.file_path) {
            local.push(track.clone());
        }
    }

    Ok(tracks)
}

/// Returns the effective music folder: configured value or OS default.
#[tauri::command]
pub async fn get_default_music_folder(
    state: State<'_, Arc<AppState>>,
) -> Result<String, String> {
    let cfg = state.config.lock();
    Ok(cfg
        .music_folder
        .clone()
        .or_else(config::default_music_folder)
        .unwrap_or_else(|| String::from(".")))
}

/// Persist a new music folder path and re-scan it immediately.
#[tauri::command]
pub async fn set_music_folder(
    path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<LocalTrack>, String> {
    {
        let mut cfg = state.config.lock();
        cfg.music_folder = Some(path.clone());
        config::save(&cfg);
    }
    let folder = PathBuf::from(&path);
    if !folder.exists() || !folder.is_dir() {
        return Err(format!("Directory not found: {}", path));
    }
    let tracks = tokio::task::spawn_blocking(move || local_library::scan_directory(&folder))
        .await
        .map_err(|e| format!("Scan failed: {}", e))?;
    let mut local = state.local_tracks.write();
    *local = tracks.clone();
    Ok(tracks)
}

/// Scan the configured music folder (or OS default) and reload the local library.
#[tauri::command]
pub async fn scan_music_folder(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<LocalTrack>, String> {
    let folder_path = {
        let cfg = state.config.lock();
        cfg.music_folder
            .clone()
            .or_else(config::default_music_folder)
            .unwrap_or_else(|| String::from("."))
    };
    let folder = PathBuf::from(&folder_path);
    if !folder.exists() || !folder.is_dir() {
        return Ok(Vec::new());
    }
    let tracks = tokio::task::spawn_blocking(move || local_library::scan_directory(&folder))
        .await
        .map_err(|e| format!("Scan failed: {}", e))?;
    let mut local = state.local_tracks.write();
    *local = tracks.clone();
    Ok(tracks)
}

#[tauri::command]
pub async fn get_local_tracks(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<LocalTrack>, String> {
    let local = state.local_tracks.read();
    Ok(local.clone())
}

#[tauri::command]
pub async fn play_local_track(
    file_path: String,
    state: State<'_, Arc<AppState>>,
) -> Result<PlaybackState, String> {
    let path = PathBuf::from(&file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }

    // Find metadata from local tracks
    let local_track = {
        let local = state.local_tracks.read();
        local.iter().find(|t| t.file_path == file_path).cloned()
    };

    let unified = if let Some(lt) = local_track {
        local_library::local_to_unified(&lt)
    } else {
        // Scan single file
        let tracks = tokio::task::spawn_blocking({
            let p = path.clone();
            move || local_library::scan_directory(p.parent().unwrap_or(&p))
        })
        .await
        .map_err(|e| format!("Scan failed: {}", e))?;

        tracks
            .iter()
            .find(|t| t.file_path == file_path)
            .map(|t| local_library::local_to_unified(t))
            .ok_or_else(|| "Failed to read track metadata".to_string())?
    };

    let mut player = state.player.write();
    player.play_file(&path, unified, None, None)?;
    Ok(player.playback_state())
}
