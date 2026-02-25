use crate::models::*;
use crate::recommendation::RecommendationEngine;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn get_queue(state: State<'_, Arc<AppState>>) -> Result<QueueState, String> {
    let queue = state.queue.read();
    let idx = state.current_index.read();
    Ok(QueueState {
        tracks: queue.iter().cloned().collect(),
        current_index: *idx,
    })
}

#[tauri::command]
pub async fn add_to_queue(
    track: UnifiedTrack,
    state: State<'_, Arc<AppState>>,
) -> Result<QueueState, String> {
    let mut queue = state.queue.write();
    queue.push_back(track);

    let idx = state.current_index.read();
    Ok(QueueState {
        tracks: queue.iter().cloned().collect(),
        current_index: *idx,
    })
}

#[tauri::command]
pub async fn clear_queue(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    state.queue.write().clear();
    *state.current_index.write() = None;
    Ok(())
}

#[tauri::command]
pub async fn smart_shuffle(
    lastfm_api_key: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<QueueState, String> {
    let engine = RecommendationEngine::new(lastfm_api_key);

    // Get current track info
    let current_track = {
        let player = state.player.read();
        player.playback_state().current_track
    };

    let mut candidates: Vec<UnifiedTrack> = Vec::new();

    // If we have a current track, try Last.fm similarity
    if let Some(track) = &current_track {
        let similar = engine
            .get_similar_tracks(&track.title, &track.artist, 30)
            .await
            .unwrap_or_default();

        // Cross-reference with Qobuz catalog
        let qobuz = {
            let guard = state.qobuz.read();
            guard.clone()
        };

        if let Some(client) = qobuz {
            for sim_track in &similar {
                let query = format!("{} {}", sim_track.name, sim_track.artist.name);
                if let Ok(results) = client.search(&query, 3).await {
                    if let Some(tracks) = results.tracks {
                        for t in tracks.items {
                            let unified = UnifiedTrack {
                                id: t.id.to_string(),
                                title: t.title,
                                artist: t
                                    .performer
                                    .as_ref()
                                    .map(|p| p.name.clone())
                                    .unwrap_or_else(|| "Unknown".to_string()),
                                album: t
                                    .album
                                    .as_ref()
                                    .map(|a| a.title.clone())
                                    .unwrap_or_else(|| "Unknown".to_string()),
                                duration_seconds: t.duration,
                                cover_url: t
                                    .album
                                    .as_ref()
                                    .and_then(|a| a.image.as_ref())
                                    .and_then(|i| i.large.clone()),
                                source: TrackSource::Qobuz { track_id: t.id },
                                quality_label: t.maximum_bit_depth.map(|bd| {
                                    format!(
                                        "{}-bit/{}kHz",
                                        bd,
                                        t.maximum_sampling_rate.unwrap_or(44.1)
                                    )
                                }),
                                sample_rate: t.maximum_sampling_rate,
                                bit_depth: t.maximum_bit_depth,
                            };
                            candidates.push(unified);
                        }
                    }
                }
            }
        }
    }

    // Also add existing queue tracks
    {
        let queue = state.queue.read();
        for t in queue.iter() {
            candidates.push(t.clone());
        }
    }

    // Get recent artists for anti-repetition
    let recent_artists: Vec<String> = {
        let queue = state.queue.read();
        queue.iter().rev().take(10).map(|t| t.artist.clone()).collect()
    };

    // Apply weighted shuffle
    let shuffled = RecommendationEngine::weighted_shuffle(&candidates, &recent_artists);

    // Replace queue
    {
        let mut queue = state.queue.write();
        queue.clear();
        for t in shuffled {
            queue.push_back(t);
        }
        *state.current_index.write() = Some(0);
    }

    let queue = state.queue.read();
    let idx = state.current_index.read();
    Ok(QueueState {
        tracks: queue.iter().cloned().collect(),
        current_index: *idx,
    })
}
