use crate::models::*;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn search(
    query: String,
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzSearchResults, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz.search(&query, 20).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_album(
    album_id: String,
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzAlbum, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz.get_album(&album_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_artist(
    artist_id: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzArtist, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz.get_artist(artist_id).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_playlist(
    playlist_id: i64,
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzPlaylist, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz
        .get_playlist(playlist_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_featured_albums(
    genre_id: Option<String>,
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzAlbumList, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz
        .get_featured_albums(genre_id.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_featured_playlists(
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzPlaylistList, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz
        .get_featured_playlists()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_genres(
    state: State<'_, Arc<AppState>>,
) -> Result<QobuzGenreList, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    qobuz.get_genres().await.map_err(|e| e.to_string())
}
