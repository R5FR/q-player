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
    /// Qobuz Connect session stop signal. Some(_) means a session is running.
    pub connect_stop: tokio::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    /// Renderers visible on the Qobuz Connect network (populated while session is active).
    pub connect_renderers: RwLock<Vec<crate::models::ConnectRenderer>>,
    /// mDNS browse daemon for discovering peer Qobuz Connect devices.
    pub connect_mdns_daemon: std::sync::Mutex<Option<mdns_sd::ServiceDaemon>>,
    /// Channel to send cast-to-renderer commands into the running Connect loop.
    pub connect_cast_tx: tokio::sync::Mutex<Option<tokio::sync::mpsc::Sender<u64>>>,
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
            connect_stop: tokio::sync::Mutex::new(None),
            connect_renderers: RwLock::new(Vec::new()),
            connect_mdns_daemon: std::sync::Mutex::new(None),
            connect_cast_tx: tokio::sync::Mutex::new(None),
        }
    }
}
