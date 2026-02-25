use crate::audio::AudioPlayer;
use crate::models::*;
use crate::qobuz::QobuzClient;
use parking_lot::RwLock;
use std::collections::VecDeque;

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
}

impl AppState {
    pub fn new() -> Self {
        Self {
            qobuz: RwLock::new(None),
            player: RwLock::new(AudioPlayer::new()),
            queue: RwLock::new(VecDeque::new()),
            current_index: RwLock::new(None),
            local_tracks: RwLock::new(Vec::new()),
            lastfm: RwLock::new(None),
            lastfm_pending_token: RwLock::new(None),
        }
    }
}
