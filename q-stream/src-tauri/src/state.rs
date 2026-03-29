use crate::audio::{AudioPlayer, PlayerEvent};
use crate::models::*;
use crate::qobuz::QobuzClient;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::{mpsc, Arc};

/// Shared application state managed by Tauri
pub struct AppState {
    pub qobuz: RwLock<Option<QobuzClient>>,
    /// Persisted user preferences (audio device, volume, EQ).
    pub config: parking_lot::Mutex<crate::config::UserConfig>,
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
    /// Channel to send controller playback commands (play/pause/seek/next/prev) when casting.
    pub connect_ctrl_tx: tokio::sync::Mutex<Option<tokio::sync::mpsc::Sender<ConnectCtrlCmd>>>,
    /// Playback state of the remote renderer while Q-Stream is inactive (cast).
    /// None when Q-Stream is the active renderer.
    pub connect_remote_state: RwLock<Option<crate::models::ConnectRemoteState>>,
    /// Q-Stream's own renderer_id as assigned by the Qobuz server.
    /// Used to exclude ourselves from the "other renderers" list.
    pub connect_own_renderer_id: RwLock<Option<u64>>,
}

/// Playback control commands sent to the active renderer when Q-Stream acts as controller.
#[derive(Debug)]
pub enum ConnectCtrlCmd {
    Play,
    Pause,
    Seek(u32),  // ms
    Next,
    Prev,
    /// Q-Stream started playing a local track: push it into the server queue
    /// so the mobile can see the title.
    LocalTrackStarted(u32),  // Qobuz track_id
}

impl AppState {
    pub fn new() -> Self {
        let (player, event_rx, spectrum) = AudioPlayer::new();
        Self {
            qobuz: RwLock::new(None),
            config: parking_lot::Mutex::new(crate::config::load()),
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
            connect_ctrl_tx: tokio::sync::Mutex::new(None),
            connect_remote_state: RwLock::new(None),
            connect_own_renderer_id: RwLock::new(None),
        }
    }
}
