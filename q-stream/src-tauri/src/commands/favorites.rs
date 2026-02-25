use crate::models::*;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;
use tracing::{info, warn};

#[tauri::command]
pub async fn get_favorites(
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzFavorites, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    info!("Fetching user favorites & playlists...");
    match qobuz.get_favorites().await {
        Ok(fav) => {
            info!(
                "Favorites loaded: {} tracks, {} albums, {} artists, {} playlists",
                fav.tracks.as_ref().map(|t| t.items.len()).unwrap_or(0),
                fav.albums.as_ref().map(|a| a.items.len()).unwrap_or(0),
                fav.artists.as_ref().map(|a| a.items.len()).unwrap_or(0),
                fav.playlists.as_ref().map(|p| p.items.len()).unwrap_or(0),
            );
            Ok(fav)
        }
        Err(e) => {
            warn!("get_favorites error: {}", e);
            Err(e.to_string())
        }
    }
}

#[tauri::command]
pub async fn add_favorite(
    item_type: String,
    item_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz
        .add_favorite(&item_type, &item_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_favorite(
    item_type: String,
    item_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz
        .remove_favorite(&item_type, &item_id)
        .await
        .map_err(|e| e.to_string())
}
