use crate::models::ArtistEnrichment;
use crate::models::QobuzAlbumSimple;
use crate::models::QobuzTrack;
use crate::musicbrainz::MusicBrainzClient;
use crate::recommendation::RecommendationEngine;
use crate::state::AppState;
use std::sync::Arc;
use tauri::State;
use tracing::info;

/// Returns Last.fm chart tracks cross-referenced with the Qobuz catalog.
/// Requires the user to be logged in (needs Qobuz client for catalog search).
#[tauri::command]
pub async fn get_trending_tracks(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<QobuzTrack>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };

    info!("Fetching Last.fm chart + cross-referencing Qobuz catalog…");
    let engine = RecommendationEngine::new(None); // uses hardcoded API key
    engine.get_trending_via_qobuz(&qobuz, 12).await
}

/// Returns Qobuz albums for the user's top Last.fm artists (last 3 months).
/// Requires the user to be logged in (needs Qobuz client for catalog search).
#[tauri::command]
pub async fn get_personalized_recommendations(
    lastfm_username: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<QobuzAlbumSimple>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };

    info!(
        "Building personalised recommendations for Last.fm user '{}'…",
        lastfm_username
    );
    let engine = RecommendationEngine::new(None);
    engine
        .get_personalized_albums(&lastfm_username, &qobuz, 12)
        .await
}

/// Returns Qobuz albums matching the user's recently played artists on Last.fm
/// (user.getRecentTracks — reflects actual listening history, not just top artists).
#[tauri::command]
pub async fn get_recent_playback_recommendations(
    lastfm_username: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<QobuzAlbumSimple>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    info!(
        "Building recent-playback recommendations for Last.fm user '{}'…",
        lastfm_username
    );
    let engine = RecommendationEngine::new(None);
    engine
        .get_recent_playback_albums(&lastfm_username, &qobuz, 12)
        .await
}

/// Discovery recommendations based on the user's Qobuz library:
/// extracts artists from favourites + playlists, then returns albums by those
/// artists that are NOT already in the library.
#[tauri::command]
pub async fn get_library_discovery(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<QobuzAlbumSimple>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    info!("Building library-based discovery recommendations…");
    let engine = RecommendationEngine::new(None);
    engine.get_library_discovery(&qobuz, 15).await
}

/// Returns the user's own playlists from Qobuz.
#[tauri::command]
pub async fn get_user_playlists(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<crate::models::QobuzPlaylist>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    info!("Fetching user playlists…");
    let pl_list = qobuz.get_user_playlists().await.map_err(|e| e.to_string())?;
    Ok(pl_list.items)
}

/// Returns MusicBrainz genre tags and a Wikipedia biography excerpt for an artist.
#[tauri::command]
pub async fn get_artist_enrichment(
    artist_name: String,
) -> Result<ArtistEnrichment, String> {
    info!("Enriching artist '{}' via MusicBrainz + Wikipedia…", artist_name);
    let mb = MusicBrainzClient::new();
    Ok(mb.enrich_artist(&artist_name).await)
}

/// Albums by the user's favorite Qobuz artists that are NOT already saved in their library.
/// Useful for surfacing new releases and B-sides from artists the user loves.
#[tauri::command]
pub async fn get_unknown_albums_by_known_artists(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<QobuzAlbumSimple>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    info!("Fetching unknown albums by favorite artists…");
    let engine = RecommendationEngine::new(None);
    engine.get_unknown_albums_by_known_artists(&qobuz, 12).await
}

/// Genre-exploration recommendations: infer the user's top genres from Last.fm
/// listening history (via MusicBrainz), then surface popular artists in those
/// genres that the user doesn't already listen to a lot.
#[tauri::command]
pub async fn get_genre_exploration(
    lastfm_username: String,
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<QobuzAlbumSimple>, String> {
    let qobuz = {
        let guard = state.qobuz.read();
        guard.clone().ok_or("Not logged in")?
    };
    info!(
        "Building genre-exploration recommendations for Last.fm user '{}'…",
        lastfm_username
    );
    let engine = RecommendationEngine::new(None);
    engine
        .get_genre_exploration(&lastfm_username, &qobuz, 12)
        .await
}
