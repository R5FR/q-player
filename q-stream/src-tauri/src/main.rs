// Q-Stream — Hi-Res Qobuz Streaming Desktop Player
// Main entry point

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod commands;
mod config;
mod lastfm;
mod local_library;
mod models;
mod musicbrainz;
mod persistence;
mod qobuz;
mod recommendation;
mod state;

use state::AppState;
use std::sync::Arc;
use tauri::Manager;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "q_stream=debug,info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let app_state = Arc::new(AppState::new());

    // Apply saved user config to the audio player at startup
    {
        let cfg = app_state.config.lock().clone();
        let mut player = app_state.player.write();
        player.set_volume(cfg.volume);
        if let Some(ref device) = cfg.audio_device {
            if device != "Default" {
                player.set_preferred_device(Some(device.clone()));
            }
        }
        if !cfg.eq_bands.is_empty() {
            player.set_eq(cfg.eq_bands.clone(), cfg.eq_enabled);
        }
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(app_state)
        .setup(|app| {
            use crate::audio::PlayerEvent;
            use tauri::Emitter;

            let state = app.state::<Arc<AppState>>().inner().clone();
            let handle = app.handle().clone();

            // Take the event receiver (can only be taken once)
            let event_rx = state
                .player_event_rx
                .lock()
                .unwrap()
                .take()
                .expect("player_event_rx already taken");

            // Bridge player events to Tauri frontend events
            std::thread::Builder::new()
                .name("player-events".into())
                .spawn(move || {
                    for event in event_rx {
                        match event {
                            PlayerEvent::TrackEnded => {
                                tracing::debug!("Emitting track-ended event to frontend");
                                handle.emit("track-ended", ()).ok();
                            }
                            PlayerEvent::Spectrum(bins) => {
                                handle.emit("spectrum-data", &bins).ok();
                            }
                        }
                    }
                    tracing::info!("Player event bridge shut down");
                })
                .expect("Failed to spawn player event bridge");

            Ok(())
        })
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
            commands::playback::play_from_queue,
            commands::playback::set_eq,
            commands::playback::get_eq_state,
            commands::playback::get_spectrum,
            commands::playback::get_audio_devices,
            commands::playback::set_audio_device,
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
            commands::queue::add_tracks_to_queue,
            commands::queue::clear_queue,
            commands::queue::play_next,
            commands::queue::remove_from_queue,
            commands::queue::smart_shuffle,
            commands::queue::enqueue_similar,
            // Local library
            commands::local_library::import_folder,
            commands::local_library::get_local_tracks,
            commands::local_library::play_local_track,
            commands::local_library::get_default_music_folder,
            commands::local_library::set_music_folder,
            commands::local_library::scan_music_folder,
            // Color extraction
            commands::ui::extract_dominant_color,
            // Recommendations
            commands::recommendations::get_trending_tracks,
            commands::recommendations::get_personalized_recommendations,
            commands::recommendations::get_recent_playback_recommendations,
            commands::recommendations::get_library_discovery,
            commands::recommendations::get_user_playlists,
            commands::recommendations::get_artist_enrichment,
            commands::recommendations::get_unknown_albums_by_known_artists,
            commands::recommendations::get_genre_exploration,
            // Persistence
            commands::persistence::load_app_data,
            commands::persistence::save_app_data,
            // Config
            commands::playback::load_config,
            // Last.fm
            commands::lastfm::lastfm_start_auth,
            commands::lastfm::lastfm_complete_auth,
            commands::lastfm::lastfm_get_session,
            commands::lastfm::lastfm_disconnect,
            commands::lastfm::lastfm_now_playing,
            commands::lastfm::lastfm_scrobble,
            // Qobuz Connect
            commands::connect::scan_connect_devices,
            commands::connect::start_qobuz_connect,
            commands::connect::stop_qobuz_connect,
            commands::connect::get_connect_status,
            commands::connect::get_connect_renderers,
            commands::connect::cast_to_renderer,
            commands::connect::control_renderer_playback,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Q-Stream");
}
