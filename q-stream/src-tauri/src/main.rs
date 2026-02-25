// Q-Stream — Hi-Res Qobuz Streaming Desktop Player
// Main entry point

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod lastfm;
mod local_library;
mod models;
mod qobuz;
mod recommendation;
mod state;

use state::AppState;
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "q_stream=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = Arc::new(AppState::new());

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .invoke_handler(tauri::generate_handler![
            // Auth
            commands::auth::login,
            commands::auth::logout,
            commands::auth::get_session,
            commands::auth::restore_session,
            // Playback
            commands::playback::play_track,
            commands::playback::pause,
            commands::playback::resume,
            commands::playback::stop,
            commands::playback::seek,
            commands::playback::set_volume,
            commands::playback::get_playback_state,
            commands::playback::next_track,
            commands::playback::previous_track,
            // Browse
            commands::browse::search,
            commands::browse::get_album,
            commands::browse::get_artist,
            commands::browse::get_playlist,
            commands::browse::get_featured_albums,
            commands::browse::get_featured_playlists,
            commands::browse::get_genres,
            // Favorites
            commands::favorites::get_favorites,
            commands::favorites::add_favorite,
            commands::favorites::remove_favorite,
            // Queue
            commands::queue::get_queue,
            commands::queue::add_to_queue,
            commands::queue::clear_queue,
            commands::queue::smart_shuffle,
            // Local library
            commands::local_library::import_folder,
            commands::local_library::get_local_tracks,
            commands::local_library::play_local_track,
            // Color extraction
            commands::ui::extract_dominant_color,
            // Recommendations
            commands::recommendations::get_trending_tracks,
            // Last.fm
            commands::lastfm::lastfm_start_auth,
            commands::lastfm::lastfm_complete_auth,
            commands::lastfm::lastfm_get_session,
            commands::lastfm::lastfm_disconnect,
            commands::lastfm::lastfm_now_playing,
            commands::lastfm::lastfm_scrobble,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Q-Stream");
}
