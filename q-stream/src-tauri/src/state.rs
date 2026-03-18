use crate::audio::{AudioPlayer, PlayerEvent};
use crate::models::*;
use crate::qobuz::QobuzClient;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::{mpsc, Arc};

/// Shared application state managed by Tauri
pub struct AppState {
    pub qobuz: RwLock<Option<QobuzClient>>,
    pub player: RwLock<AudioPlayer>,
    pub queue: RwLock<VecDeque<UnifiedTrack>>,
    pub current_index: RwLock<Option<usize>>,
    pub local_tracks: RwLock<Vec<LocalTrack>>,
    /// Active Last.fm user session (restored from disk on startup)
    pub lastfm: RwLock<Option<LastFmUserSession>>,
    /// Transient token waiting for user to authorize in browser
    pub lastfm_pending_token: RwLock<Option<String>>,
    /// Receiver for player events (taken once during setup)
    pub player_event_rx: std::sync::Mutex<Option<mpsc::Receiver<PlayerEvent>>>,
    /// Direct reference to the spectrum buffer — bypasses the player RwLock
    /// so get_spectrum never blocks behind play/seek write locks.
    pub spectrum: Arc<parking_lot::Mutex<Vec<f32>>>,
}

impl AppState {
    pub fn new() -> Self {
        let (player, event_rx, spectrum) = AudioPlayer::new();
        Self {
            qobuz: RwLock::new(None),
            player: RwLock::new(player),
            queue: RwLock::new(VecDeque::new()),
            current_index: RwLock::new(None),
            local_tracks: RwLock::new(Vec::new()),
            lastfm: RwLock::new(None),
            lastfm_pending_token: RwLock::new(None),
            player_event_rx: std::sync::Mutex::new(Some(event_rx)),
            spectrum,
        }
    }
}
