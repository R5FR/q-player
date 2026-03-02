use crate::models::*;
use crate::recommendation::RecommendationEngine;
use crate::state::AppState;
use std::collections::HashSet;
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

/// Add multiple tracks to the queue at once (e.g. entire album/playlist).
#[tauri::command]
pub async fn add_tracks_to_queue(
    tracks: Vec<UnifiedTrack>,
    state: State<'_, Arc<AppState>>,
) -> Result<QueueState, String> {
    let mut queue = state.queue.write();
    for track in tracks {
        queue.push_back(track);
    }

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

/// Insert a track right after the currently playing position.
/// If nothing is playing, inserts at the front.
#[tauri::command]
pub async fn play_next(
    track: UnifiedTrack,
    state: State<'_, Arc<AppState>>,
) -> Result<QueueState, String> {
    let mut queue = state.queue.write();
    let idx = state.current_index.read();
    let insert_pos = match *idx {
        Some(i) => (i + 1).min(queue.len()),
        None => 0,
    };
    queue.insert(insert_pos, track);

    Ok(QueueState {
        tracks: queue.iter().cloned().collect(),
        current_index: *idx,
    })
}

/// Remove a single track from the queue by index, adjusting current_index
/// as needed.
#[tauri::command]
pub async fn remove_from_queue(
    index: usize,
    state: State<'_, Arc<AppState>>,
) -> Result<QueueState, String> {
    let mut queue = state.queue.write();
    if index >= queue.len() {
        return Err(format!(
            "Index {} out of bounds (queue len={})",
            index,
            queue.len()
        ));
    }
    queue.remove(index);

    // Adjust current_index
    let mut idx = state.current_index.write();
    if let Some(current) = *idx {
        if index < current {
            *idx = Some(current - 1);
        } else if index == current {
            // Current track was removed
            if queue.is_empty() {
                *idx = None;
            } else {
                *idx = Some(current.min(queue.len() - 1));
            }
        }
    }

    Ok(QueueState {
        tracks: queue.iter().cloned().collect(),
        current_index: *idx,
    })
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

/// Append Last.fm-similar tracks for the given track to the END of the current queue.
/// Cross-references candidates with the Qobuz catalog.
/// Returns the number of tracks added.
#[tauri::command]
pub async fn enqueue_similar(
    track_title: String,
    track_artist: String,
    state: State<'_, Arc<AppState>>,
) -> Result<usize, String> {
    let engine = RecommendationEngine::new(None);

    let similar = engine
        .get_similar_tracks(&track_title, &track_artist, 30)
        .await
        .unwrap_or_default();

    if similar.is_empty() {
        return Ok(0);
    }

    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };

    // Build a set of titles already in the queue to avoid duplicates
    let existing_titles: HashSet<String> = {
        let queue = state.queue.read();
        queue.iter().map(|t| t.title.to_lowercase()).collect()
    };

    let mut added = 0usize;

    for sim_track in &similar {
        if added >= 10 {
            break;
        }
        let query = format!("{} {}", sim_track.name, sim_track.artist.name);
        if let Ok(results) = qobuz.search(&query, 3).await {
            if let Some(tracks) = results.tracks {
                for t in tracks.items {
                    let title_lower = t.title.to_lowercase();
                    if existing_titles.contains(&title_lower) {
                        continue;
                    }
                    let unified = UnifiedTrack {
                        id: t.id.to_string(),
                        title: t.title.clone(),
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
                    state.queue.write().push_back(unified);
                    added += 1;
                    break; // One Qobuz match per Last.fm suggestion
                }
            }
        }
    }

    Ok(added)
}
