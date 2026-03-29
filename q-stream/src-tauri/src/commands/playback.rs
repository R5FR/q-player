use crate::audio::EqBandParam;
use crate::models::*;
use crate::state::{AppState, ConnectCtrlCmd};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn play_track(
    track_id: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<PlaybackState, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };

    // Fetch URL and metadata concurrently
    let (track_url_res, track_info_res) = tokio::join!(
        qobuz.get_track_url(track_id),
        qobuz.get_track(track_id),
    );
    let track_url = track_url_res.map_err(|e| format!("Failed to get track URL: {e}"))?;
    let track_info = track_info_res.map_err(|e| format!("Failed to get track info: {e}"))?;

    // Fetch bytes (from cache or network)
    let bytes = qobuz
        .fetch_track_bytes(&track_url)
        .await
        .map_err(|e| format!("Failed to fetch track: {e}"))?;

    let unified = UnifiedTrack {
        id: track_id.to_string(),
        title: track_info.title,
        artist: track_info.performer.as_ref().map(|p| p.name.clone()).unwrap_or_else(|| "Unknown".into()),
        album: track_info.album.as_ref().map(|a| a.title.clone()).unwrap_or_else(|| "Unknown".into()),
        duration_seconds: track_info.duration,
        cover_url: track_info.album.as_ref().and_then(|a| a.image.as_ref()).and_then(|i| i.large.clone()),
        source: TrackSource::Qobuz { track_id },
        quality_label: Some(format!("{}-bit/{}kHz FLAC", track_url.bit_depth, track_url.sampling_rate)),
        sample_rate: Some(track_url.sampling_rate),
        bit_depth: Some(track_url.bit_depth),
    };

    {
        let mut player = state.player.write();
        player.play_bytes(bytes, unified, Some(track_url.sampling_rate), Some(track_url.bit_depth))?;
    }

    {
        let ctrl = state.connect_ctrl_tx.lock().await;
        if let Some(tx) = ctrl.as_ref() {
            let _ = tx.try_send(ConnectCtrlCmd::LocalTrackStarted(track_id as u32));
        }
    }

    Ok(state.player.read().playback_state())
}

#[tauri::command]
pub async fn pause(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.player.write().pause();
    Ok(())
}

#[tauri::command]
pub async fn resume(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.player.write().resume();
    Ok(())
}

#[tauri::command]
pub async fn stop(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.player.write().stop();
    Ok(())
}

#[tauri::command]
pub async fn seek(position_ms: u64, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.player.write().seek(position_ms)
}

#[tauri::command]
pub async fn set_volume(volume: f32, state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.player.write().set_volume(volume);
    {
        let mut cfg = state.config.lock();
        cfg.volume = volume;
        crate::config::save(&cfg);
    }
    Ok(())
}

#[tauri::command]
pub async fn get_playback_state(
    state: State<'_, Arc<AppState>>,
) -> Result<PlaybackState, String> {
    let mut pb = state.player.read().playback_state();

    // If a remote renderer is active (Q-Stream has cast), overlay its state.
    if let Some(remote) = state.connect_remote_state.read().as_ref() {
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let elapsed = now_ms.saturating_sub(remote.last_updated_at_ms);
        let interp_pos = if remote.is_playing {
            (remote.position_ms + elapsed).min(remote.duration_ms)
        } else {
            remote.position_ms
        };
        pb.is_playing = remote.is_playing;
        pb.position_ms = interp_pos;
        pb.duration_ms = remote.duration_ms;
        if remote.track.is_some() {
            pb.current_track = remote.track.clone();
        }
    }

    Ok(pb)
}

#[tauri::command]
pub async fn next_track(state: State<'_, Arc<AppState>>) -> Result<Option<PlaybackState>, String> {
    let next_track = {
        let queue = state.queue.read();
        let mut idx = state.current_index.write();
        let current = idx.unwrap_or(0);
        let next = current + 1;
        if next < queue.len() {
            *idx = Some(next);
            Some(queue[next].clone())
        } else {
            None
        }
    };

    if let Some(track) = next_track {
        Ok(Some(play_qobuz_track(state, track).await?))
    } else {
        Ok(None)
    }
}

#[tauri::command]
pub async fn previous_track(
    state: State<'_, Arc<AppState>>,
) -> Result<Option<PlaybackState>, String> {
    let prev_track = {
        let queue = state.queue.read();
        let mut idx = state.current_index.write();
        let current = idx.unwrap_or(0);
        if current > 0 {
            let prev = current - 1;
            *idx = Some(prev);
            Some(queue[prev].clone())
        } else {
            None
        }
    };

    if let Some(track) = prev_track {
        Ok(Some(play_qobuz_track(state, track).await?))
    } else {
        Ok(None)
    }
}

/// Internal helper: fetch + play a Qobuz track from a `UnifiedTrack` already in the queue.
/// Metadata is already known; only the stream URL and bytes are fetched.
async fn play_qobuz_track(
    state: State<'_, Arc<AppState>>,
    track: UnifiedTrack,
) -> Result<PlaybackState, String> {
    match &track.source {
        TrackSource::Qobuz { track_id } => {
            let tid = *track_id;
            let qobuz = {
                let guard = state.qobuz.read();
                guard.clone().ok_or("Not logged in")?
            };
            let track_url = qobuz.get_track_url(tid).await.map_err(|e| e.to_string())?;
            let bytes = qobuz.fetch_track_bytes(&track_url).await.map_err(|e| e.to_string())?;
            let pb = {
                let mut player = state.player.write();
                player.play_bytes(bytes, track, Some(track_url.sampling_rate), Some(track_url.bit_depth))?;
                player.playback_state()
            };
            let ctrl = state.connect_ctrl_tx.lock().await;
            if let Some(tx) = ctrl.as_ref() {
                let _ = tx.try_send(ConnectCtrlCmd::LocalTrackStarted(tid as u32));
            }
            Ok(pb)
        }
        TrackSource::Local { file_path } => {
            let path = PathBuf::from(file_path);
            let mut player = state.player.write();
            player.play_file(&path, track, None, None)?;
            Ok(player.playback_state())
        }
    }
}

// ── EQ Commands ─────────────────────────────────────────────────────

/// Set equalizer bands and enabled state. Immediately applied to the audio pipeline.
#[tauri::command]
pub async fn set_eq(
    bands: Vec<EqBandParam>,
    enabled: bool,
    advanced: bool,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.player.write().set_eq(bands.clone(), enabled);
    {
        let mut cfg = state.config.lock();
        cfg.eq_bands = bands;
        cfg.eq_enabled = enabled;
        cfg.eq_advanced = advanced;
        crate::config::save(&cfg);
    }
    Ok(())
}

/// Return current EQ configuration (for UI synchronisation on startup).
#[tauri::command]
pub async fn get_eq_state(
    state: State<'_, Arc<AppState>>,
) -> Result<serde_json::Value, String> {
    let (enabled, bands) = state.player.read().get_eq_state();
    Ok(serde_json::json!({ "enabled": enabled, "bands": bands }))
}

/// Returns the latest FFT spectrum data (80 bins, 0.0–1.0 normalized).
/// Reads directly from the shared spectrum Arc — never blocks on the player RwLock.
/// The frontend detects staleness by comparing consecutive values.
#[tauri::command]
pub async fn get_spectrum(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<f32>, String> {
    Ok(state.spectrum.lock().clone())
}

// ── Audio Device Commands ────────────────────────────────────────────

/// List all available audio output devices on the host system.
#[tauri::command]
pub async fn get_audio_devices() -> Result<Vec<String>, String> {
    Ok(crate::audio::AudioPlayer::get_audio_devices())
}

/// Switch the audio output to a specific device (null = system default).
/// Interrupts current playback; the user must press play to resume.
#[tauri::command]
pub async fn set_audio_device(
    device_name: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    state.player.write().set_preferred_device(device_name.clone());
    {
        let mut cfg = state.config.lock();
        cfg.audio_device = device_name.or(Some("Default".to_string()));
        crate::config::save(&cfg);
    }
    Ok(())
}

/// Return the saved user config so the frontend can restore its UI state on startup.
#[tauri::command]
pub async fn load_config(
    state: State<'_, Arc<AppState>>,
) -> Result<crate::config::UserConfig, String> {
    Ok(state.config.lock().clone())
}

/// Play the track at a given queue index, reusing the metadata already stored in the queue entry.
/// Unlike `play_track`, this does NOT perform a Qobuz catalog search — only the stream URL and
/// audio bytes are fetched. Use this after building a queue with `add_to_queue` / `clear_queue`.
#[tauri::command]
pub async fn play_from_queue(
    idx: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<PlaybackState, String> {
    let track = {
        let queue = state.queue.read();
        if idx >= queue.len() {
            return Err(format!(
                "Queue index {} out of bounds (queue len={})",
                idx,
                queue.len()
            ));
        }
        *state.current_index.write() = Some(idx);
        queue[idx].clone()
    };

    match &track.source {
        TrackSource::Qobuz { track_id } => {
            let qobuz = {
                let guard = state.qobuz.read();
                guard.clone().ok_or("Not logged in")?
            };

            let track_url = qobuz
                .get_track_url(*track_id)
                .await
                .map_err(|e| e.to_string())?;

            let bytes = qobuz
                .fetch_track_bytes(&track_url)
                .await
                .map_err(|e| e.to_string())?;

            let mut player = state.player.write();
            player.play_bytes(
                bytes,
                track,
                Some(track_url.sampling_rate),
                Some(track_url.bit_depth),
            )?;

            Ok(player.playback_state())
        }
        TrackSource::Local { file_path } => {
            let path = PathBuf::from(file_path);
            let mut player = state.player.write();
            player.play_file(&path, track, None, None)?;
            Ok(player.playback_state())
        }
    }
}
