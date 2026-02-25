use crate::models::QobuzTrack;
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
