use std::hash::{Hash, Hasher};
use std::sync::Arc;

use mdns_sd::ServiceEvent;
use crate::qonductor::{
    msg, ActivationState, BufferState, Command, DeviceConfig, Notification, PlayingState,
    SessionEvent, SessionInfo, SessionManager,
    msg::{PositionExt, QueueRendererStateExt, SetStateExt},
};
use tauri::State;
use tracing::{error, info, warn};

use crate::models::{ConnectRemoteState, ConnectRenderer, TrackSource, UnifiedTrack};
use crate::state::{AppState, ConnectCtrlCmd};

fn hash_str(s: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    hasher.finish()
}

/// Start a background mDNS browse for `_qobuz-connect._tcp.local.` peers.
/// Discovered devices are added to `state.connect_renderers`.
fn start_mdns_browse(state: Arc<AppState>) {
    // Stop any previous browse daemon
    {
        let mut guard = state.connect_mdns_daemon.lock().unwrap();
        if let Some(old) = guard.take() {
            let _ = old.shutdown();
        }
    }

    let mdns = match mdns_sd::ServiceDaemon::new() {
        Ok(d) => d,
        Err(e) => {
            error!("mDNS browse: failed to create daemon: {e}");
            return;
        }
    };

    let receiver = match mdns.browse("_qobuz-connect._tcp.local.") {
        Ok(r) => r,
        Err(e) => {
            error!("mDNS browse: failed to start: {e}");
            return;
        }
    };

    {
        let mut guard = state.connect_mdns_daemon.lock().unwrap();
        *guard = Some(mdns);
    }

    let state_clone = state.clone();
    std::thread::Builder::new()
        .name("qconnect-mdns".into())
        .spawn(move || {
            for event in receiver.into_iter() {
                match event {
                    ServiceEvent::ServiceResolved(info) => {
                        let fullname = info.get_fullname().to_string();

                        // Extract instance name from fullname: "Device._qobuz-connect._tcp.local." → "Device"
                        let instance_name = fullname
                            .split("._qobuz-connect")
                            .next()
                            .unwrap_or(&fullname)
                            .to_string();

                        // Try multiple TXT record key variants used by different Qobuz clients
                        let name = info
                            .get_property_val_str("Name")
                            .or_else(|| info.get_property_val_str("fn"))
                            .or_else(|| info.get_property_val_str("name"))
                            .map(|s| s.to_string())
                            .unwrap_or_else(|| instance_name.clone());

                        // Skip our own advertised device
                        if name == "Q-Stream" || instance_name == "Q-Stream" {
                            continue;
                        }

                        let model = info
                            .get_property_val_str("type")
                            .or_else(|| info.get_property_val_str("md"))
                            .unwrap_or("Qobuz Connect")
                            .to_string();

                        let renderer_id = hash_str(&fullname);
                        info!("mDNS: discovered '{name}' [{model}]");

                        let mut list = state_clone.connect_renderers.write();
                        if !list.iter().any(|x| x.renderer_id == renderer_id) {
                            list.push(ConnectRenderer {
                                renderer_id,
                                name,
                                model,
                                is_active: false,
                            });
                        }
                    }
                    ServiceEvent::ServiceRemoved(_, fullname) => {
                        let renderer_id = hash_str(&fullname);
                        state_clone
                            .connect_renderers
                            .write()
                            .retain(|x| x.renderer_id != renderer_id);
                    }
                    _ => {}
                }
            }
            info!("mDNS browse thread ended");
        })
        .ok();
}

// ── Local session state ─────────────────────────────────────────────────────

/// State tracked locally by the Connect event loop.
/// Mirrors what the Qobuz server knows about us.
struct ConnectState {
    /// Full queue as received from the server.
    queue: Vec<msg::QueueTrackRef>,
    /// Track ID currently loaded / playing.
    current_track_id: Option<u32>,
    /// Queue item ID on the Qobuz server corresponding to the current track.
    /// Required for the mobile app to show track metadata.
    current_queue_item_id: Option<i32>,
    /// Playback state we last reported to the server.
    playing: PlayingState,
    /// Position in ms we last reported to the server.
    position_ms: u32,
    /// Duration in ms of the current track.
    duration_ms: u32,
    /// Pending play: (queue_item_id, seek_ms) when SetState arrived before QueueLoadTracks.
    pending_play: Option<(u64, Option<u32>)>,
    /// track_id the remote renderer is currently playing (tracked via RestoreState).
    last_remote_track_id: Option<u32>,
    /// Suppress the first SetState-triggered auto-play after activation.
    /// The Qobuz server always sends a restoration SetState right after SetActive,
    /// replaying the previous session's track. We skip this initial auto-play;
    /// any subsequent user-initiated action will clear this flag.
    suppress_initial_play: bool,
}

impl ConnectState {
    fn new() -> Self {
        Self {
            queue: Vec::new(),
            current_track_id: None,
            current_queue_item_id: None,
            playing: PlayingState::Stopped,
            position_ms: 0,
            duration_ms: 0,
            pending_play: None,
            last_remote_track_id: None,
            suppress_initial_play: true,
        }
    }

    fn renderer_state(&self) -> msg::QueueRendererState {
        let mut s = msg::QueueRendererState {
            current_position: Some(msg::Position::now(self.position_ms)),
            duration: if self.duration_ms > 0 { Some(self.duration_ms) } else { None },
            current_queue_item_id: self.current_queue_item_id,
            ..Default::default()
        };
        s.set_state(self.playing).set_buffer(BufferState::Ok);
        s
    }
}

// ── Tauri commands ──────────────────────────────────────────────────────────

/// Scan for Qobuz Connect devices on the local network via mDNS.
/// Restarts the browse (sends a fresh PTR query) so recently-online devices appear quickly.
/// Safe to call at any time — independently of the Connect session.
#[tauri::command]
pub async fn scan_connect_devices(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    start_mdns_browse(state.inner().clone());
    Ok(())
}

/// Start a Qobuz Connect session. The device will appear in the Qobuz mobile app.
#[tauri::command]
pub async fn start_qobuz_connect(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let app_id = {
        let qobuz = state.qobuz.read();
        match qobuz.as_ref() {
            Some(q) => q.app_id().to_string(),
            None => return Err("Not logged in to Qobuz".to_string()),
        }
    };

    // Stop any running session first
    {
        let mut stop = state.connect_stop.lock().await;
        if let Some(tx) = stop.take() {
            let _ = tx.send(());
        }
    }

    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    {
        let mut stop = state.connect_stop.lock().await;
        *stop = Some(stop_tx);
    }

    let state_clone = state.inner().clone();
    tokio::spawn(async move {
        if let Err(e) = run_connect_loop(state_clone, app_id, stop_rx).await {
            error!("Qobuz Connect error: {e}");
        }
        info!("Qobuz Connect session ended");
    });

    Ok(())
}

/// Stop the running Qobuz Connect session.
#[tauri::command]
pub async fn stop_qobuz_connect(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let mut stop = state.connect_stop.lock().await;
    match stop.take() {
        Some(tx) => {
            let _ = tx.send(());
            Ok(())
        }
        None => Err("Qobuz Connect is not running".to_string()),
    }
}

/// Returns true if a Qobuz Connect session is active.
#[tauri::command]
pub async fn get_connect_status(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    Ok(state.connect_stop.lock().await.is_some())
}

/// Returns the list of Qobuz Connect renderers currently visible on the network.
#[tauri::command]
pub async fn get_connect_renderers(
    state: State<'_, Arc<AppState>>,
) -> Result<Vec<ConnectRenderer>, String> {
    let own_id = *state.connect_own_renderer_id.read();
    let list = state.connect_renderers.read();
    Ok(list
        .iter()
        .filter(|r| own_id.map_or(true, |id| r.renderer_id != id))
        .cloned()
        .collect())
}

/// Control the active Qobuz Connect renderer (play / pause / seek / next / prev).
/// Only meaningful after Q-Stream has cast to another renderer.
#[tauri::command]
pub async fn control_renderer_playback(
    state: State<'_, Arc<AppState>>,
    action: String,
    position_ms: Option<u32>,
) -> Result<(), String> {
    let cmd = match action.as_str() {
        "play"  => ConnectCtrlCmd::Play,
        "pause" => ConnectCtrlCmd::Pause,
        "seek"  => ConnectCtrlCmd::Seek(position_ms.unwrap_or(0)),
        "next"  => ConnectCtrlCmd::Next,
        "prev"  => ConnectCtrlCmd::Prev,
        other   => return Err(format!("Unknown action: {other}")),
    };
    let guard = state.connect_ctrl_tx.lock().await;
    match &*guard {
        Some(tx) => tx.send(cmd).await.map_err(|e| format!("Ctrl channel closed: {e}")),
        None => Err("Qobuz Connect is not running".to_string()),
    }
}

/// Transfer playback back to Q-Stream (undo a previous cast).
/// Uses Q-Stream's own renderer_id received at registration.
#[tauri::command]
pub async fn cast_to_own_renderer(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    let own_id = *state.connect_own_renderer_id.read();
    let own_id = own_id.ok_or("Q-Stream renderer ID not yet assigned — Connect session may still be starting")?;
    let guard = state.connect_cast_tx.lock().await;
    match &*guard {
        Some(tx) => tx
            .send(own_id)
            .await
            .map_err(|e| format!("Cast channel closed: {e}")),
        None => Err("Qobuz Connect is not running".to_string()),
    }
}

/// Transfer playback to an external Qobuz Connect renderer.
/// The Qobuz server will deactivate Q-Stream and activate the target renderer.
#[tauri::command]
pub async fn cast_to_renderer(
    state: State<'_, Arc<AppState>>,
    renderer_id: u64,
) -> Result<(), String> {
    let guard = state.connect_cast_tx.lock().await;
    match &*guard {
        Some(tx) => tx
            .send(renderer_id)
            .await
            .map_err(|e| format!("Cast channel closed: {e}")),
        None => Err("Qobuz Connect is not running".to_string()),
    }
}

// ── Session loop ────────────────────────────────────────────────────────────

async fn run_connect_loop(
    state: Arc<AppState>,
    app_id: String,
    mut stop_rx: tokio::sync::oneshot::Receiver<()>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut manager = SessionManager::start(0).await?; // port 0 = OS-assigned
    let mut session = manager
        .add_device(DeviceConfig::new("Q-Stream", &app_id))
        .await?;
    info!("Qobuz Connect: device 'Q-Stream' registered");

    // Proactively open a WebSocket session so the Qobuz server pushes AddRenderer
    // notifications for all online devices (including the phone/controller).
    {
        let qobuz = state.qobuz.read().clone();
        if let Some(ref q) = qobuz {
            match q.get_connect_jwt().await {
                Ok((ws_endpoint, ws_jwt)) => {
                    let si = SessionInfo {
                        session_id: uuid::Uuid::new_v4().to_string(),
                        ws_endpoint,
                        ws_jwt,
                        ws_jwt_exp: 0,
                        api_jwt: String::new(),
                        api_jwt_exp: 0,
                    };
                    if let Err(e) = manager.connect_proactive(si).await {
                        warn!("Proactive Connect failed: {e}");
                    } else {
                        info!("Qobuz Connect: proactive WebSocket session started");
                    }
                }
                Err(e) => warn!("Could not fetch Connect JWT: {e}"),
            }
        }
    }

    tokio::spawn(async move {
        if let Err(e) = manager.run().await {
            warn!("Connect manager error: {e}");
        }
    });

    // Set up cast-to channel so Tauri commands can transfer playback to another renderer
    let (cast_tx, mut cast_rx) = tokio::sync::mpsc::channel::<u64>(8);
    {
        let mut guard = state.connect_cast_tx.lock().await;
        *guard = Some(cast_tx);
    }

    // Set up controller-command channel for controlling the active renderer after cast
    let (ctrl_tx, mut ctrl_rx) = tokio::sync::mpsc::channel::<ConnectCtrlCmd>(8);
    {
        let mut guard = state.connect_ctrl_tx.lock().await;
        *guard = Some(ctrl_tx);
    }

    let mut connect = ConnectState::new();

    loop {
        tokio::select! {
            _ = &mut stop_rx => {
                info!("Qobuz Connect: stop signal received");
                break;
            }
            event = session.recv() => {
                match event {
                    Some(e) => handle_event(&state, &mut connect, e).await,
                    None => {
                        info!("Qobuz Connect: session channel closed");
                        break;
                    }
                }
            }
            Some(renderer_id) = cast_rx.recv() => {
                info!(
                    "Qobuz Connect: casting to renderer {renderer_id} \
                     (current: track_id={:?} queue_item_id={:?} pos={}ms playing={:?})",
                    connect.current_track_id,
                    connect.current_queue_item_id,
                    connect.position_ms,
                    connect.playing,
                );
                if let Err(e) = session.cast_to(renderer_id).await {
                    warn!("Cast to renderer {renderer_id} failed: {e}");
                } else {
                    info!("Qobuz Connect: cast command sent — staying connected as inactive renderer");
                }
            }
            Some(cmd) = ctrl_rx.recv() => {
                match cmd {
                    ConnectCtrlCmd::LocalTrackStarted(track_id) => {
                        info!("Connect: local track started (track_id={track_id}), pushing to server queue");
                        connect.current_track_id = Some(track_id);
                        connect.current_queue_item_id = None;
                        connect.playing = PlayingState::Playing;
                        connect.suppress_initial_play = false;
                        if let Err(e) = session.push_queue(vec![track_id]).await {
                            warn!("Connect: push_queue failed: {e}");
                        }
                    }
                    ConnectCtrlCmd::LocalPaused => {
                        connect.playing = PlayingState::Paused;
                        let pb = state.player.read().playback_state();
                        connect.position_ms = pb.position_ms as u32;
                    }
                    ConnectCtrlCmd::LocalResumed => {
                        connect.playing = PlayingState::Playing;
                        connect.suppress_initial_play = false;
                    }
                    ConnectCtrlCmd::LocalSeeked(ms) => {
                        connect.position_ms = ms;
                    }
                    other => {
                        handle_ctrl_cmd(&session, &mut connect, other).await;
                    }
                }
            }
        }
    }

    // Cleanup on exit — mDNS browse runs independently, keep renderers list intact
    {
        let mut cast = state.connect_cast_tx.lock().await;
        *cast = None;
    }
    {
        let mut ctrl = state.connect_ctrl_tx.lock().await;
        *ctrl = None;
    }
    *state.connect_remote_state.write() = None;
    let mut stop = state.connect_stop.lock().await;
    *stop = None;

    Ok(())
}

// ── Event handler ───────────────────────────────────────────────────────────

async fn handle_event(state: &Arc<AppState>, connect: &mut ConnectState, event: SessionEvent) {
    match event {
        SessionEvent::Command(cmd) => match cmd {
            // Device activated by Qobuz app: report current state
            Command::SetActive { respond, .. } => {
                info!(">>> CMD SetActive (device activated by Qobuz app)");
                // Clear remote state: Q-Stream is now the active renderer again
                *state.connect_remote_state.write() = None;
                // Suppress the upcoming server restoration SetState (previous session replay).
                connect.suppress_initial_play = true;
                let pb = state.player.read().playback_state();
                respond.send(ActivationState {
                    muted: false,
                    volume: (pb.volume * 100.0) as u32,
                    max_quality: 4, // HiRes 192kHz
                    playback: connect.renderer_state(),
                });
            }

            // Periodic heartbeat: report current position + duration
            Command::Heartbeat { respond } => {
                if connect.playing == PlayingState::Playing {
                    let pb = state.player.read().playback_state();
                    connect.position_ms = pb.position_ms as u32;
                    connect.duration_ms = pb.duration_ms as u32;
                    respond.send(Some(connect.renderer_state()));
                } else {
                    respond.send(None);
                }
            }

            // Play / pause / seek / track change
            Command::SetState { cmd, respond } => {
                let explicit_state = cmd.state();
                let new_state = explicit_state.unwrap_or(connect.playing);
                let position_ms = cmd.current_position;
                info!(
                    ">>> CMD SetState  playing_state={:?}  position_ms={:?}  queue_item_id={:?}  track_id={:?}  next_queue_item_id={:?}",
                    cmd.playing_state,
                    position_ms,
                    cmd.current_queue_item.as_ref().and_then(|q| q.queue_item_id),
                    cmd.current_queue_item.as_ref().and_then(|q| q.track_id),
                    cmd.next_queue_item.as_ref().and_then(|q| q.queue_item_id),
                );

                // Filter Qobuz sentinel values (u32::MAX / u64::MAX = "no track")
                let queue_item_id_raw = cmd.current_queue_item.as_ref()
                    .and_then(|q| q.queue_item_id)
                    .filter(|&id| id != u64::MAX);
                let queue_item_id = queue_item_id_raw.map(|id| id as i32);

                // track_id may be absent (direct tap from phone album view sends only queue_item_id).
                // Fall back to looking up in our cached queue by queue_item_id.
                let requested_track_id = cmd.current_queue_item.as_ref()
                    .and_then(|q| q.track_id)
                    .filter(|&id| id != u32::MAX)
                    .or_else(|| {
                        queue_item_id_raw.and_then(|qid| {
                            connect.queue.iter()
                                .find(|t| t.queue_item_id == Some(qid))
                                .and_then(|t| t.track_id)
                        })
                    });

                info!(
                    "    → resolved: new_state={:?}  requested_track_id={:?}  queue_cache_len={}  current_track_id={:?}  current_queue_item_id={:?}",
                    new_state,
                    requested_track_id,
                    connect.queue.len(),
                    connect.current_track_id,
                    connect.current_queue_item_id,
                );

                // A track change is detected either by a new track_id OR a new queue_item_id
                // (same track replayed from a different queue position counts as a new play).
                let is_new_track = requested_track_id.is_some()
                    && requested_track_id != connect.current_track_id;
                let is_new_queue_item = queue_item_id.is_some()
                    && queue_item_id != connect.current_queue_item_id;
                let should_load_new = is_new_track || (is_new_queue_item && requested_track_id.is_some());

                // Only Playing (or absent) state means "user wants to play".
                // Paused / Stopped should not trigger a new load or deferred-play.
                let will_play = matches!(explicit_state, Some(PlayingState::Playing) | None);

                info!(
                    "    → is_new_track={is_new_track}  is_new_queue_item={is_new_queue_item}  should_load_new={should_load_new}  will_play={will_play}"
                );

                if should_load_new && will_play {
                    // Suppress the very first server-initiated play after activation.
                    // The server always sends a restoration SetState to replay the previous
                    // session's track right after SetActive — we skip this silently.
                    if connect.suppress_initial_play {
                        info!("Connect: suppressing initial auto-play restoration (track_id={:?})", requested_track_id);
                        connect.suppress_initial_play = false;
                        connect.current_track_id = requested_track_id;
                        connect.current_queue_item_id = queue_item_id;
                        connect.playing = PlayingState::Stopped;
                        respond.send(connect.renderer_state());
                        return;
                    }

                    // New track requested — respond with Buffering immediately,
                    // then fetch + play in a background task.
                    connect.current_track_id = requested_track_id;
                    connect.current_queue_item_id = queue_item_id;
                    connect.playing = PlayingState::Playing;
                    connect.pending_play = None;
                    if let Some(pos) = position_ms {
                        connect.position_ms = pos;
                    }

                    let mut s = msg::QueueRendererState { ..Default::default() };
                    s.set_state(PlayingState::Playing)
                        .set_buffer(BufferState::Buffering);
                    respond.send(s);

                    let track_id = requested_track_id.unwrap() as i64;
                    let state_clone = state.clone();
                    let seek_to = position_ms;
                    tokio::spawn(async move {
                        if let Err(e) = fetch_and_play(state_clone, track_id, seek_to).await {
                            error!("Connect: failed to play track {track_id}: {e}");
                        }
                    });
                } else if will_play && queue_item_id_raw.is_some() && requested_track_id.is_none() {
                    // We have a queue_item_id but couldn't resolve track_id yet —
                    // QueueLoadTracks hasn't arrived yet (race). Store as pending.
                    let qid = queue_item_id_raw.unwrap();
                    info!("Connect: SetState queue_item_id={qid} not in queue yet — deferring");
                    connect.pending_play = Some((qid, position_ms));
                    // Respond with current state so the server doesn't time out.
                    respond.send(connect.renderer_state());
                } else {
                    // Same track: play/pause/seek/stop — user is interacting, clear suppression
                    connect.suppress_initial_play = false;
                    match new_state {
                        PlayingState::Playing => {
                            state.player.write().resume();
                            if let Some(pos) = position_ms {
                                let _ = state.player.write().seek(pos as u64);
                                connect.position_ms = pos;
                            }
                        }
                        PlayingState::Paused => {
                            state.player.write().pause();
                            if let Some(pos) = position_ms {
                                connect.position_ms = pos;
                            }
                        }
                        PlayingState::Stopped => {
                            state.player.write().stop();
                            connect.position_ms = 0;
                        }
                        _ => {}
                    }
                    connect.playing = new_state;
                    respond.send(connect.renderer_state());
                }
            }
        },

        SessionEvent::Notification(n) => match n {
            // Full queue snapshot (type 90)
            Notification::QueueState(queue) => {
                info!(">>> NOTIF QueueState (full snapshot, {} tracks)", queue.tracks.len());
                // Log first 5 tracks to see if queue_item_ids are populated
                for (i, t) in queue.tracks.iter().take(5).enumerate() {
                    info!("    track[{i}]: queue_item_id={:?} track_id={:?}", t.queue_item_id, t.track_id);
                }
                if let Some(track_id) = connect.current_track_id {
                    if let Some(item) = queue.tracks.iter().find(|t| t.track_id == Some(track_id)) {
                        connect.current_queue_item_id = item.queue_item_id.map(|id| id as i32);
                    }
                }
                connect.queue = queue.tracks;
                // Resolve any pending play deferred before the queue was ready
                if let Some((pending_qid, pending_pos)) = connect.pending_play.take() {
                    if let Some(item) = connect.queue.iter().find(|t| t.queue_item_id == Some(pending_qid)) {
                        if let Some(track_id) = item.track_id {
                            info!("Connect: resolving deferred play (from QueueState) for queue_item_id={pending_qid} → track_id={track_id}");
                            connect.current_track_id = Some(track_id);
                            connect.current_queue_item_id = Some(pending_qid as i32);
                            connect.playing = PlayingState::Playing;
                            let state_clone = state.clone();
                            tokio::spawn(async move {
                                if let Err(e) = fetch_and_play(state_clone, track_id as i64, pending_pos).await {
                                    error!("Connect: deferred fetch_and_play failed: {e}");
                                }
                            });
                        }
                    }
                }
            }

            // Queue replaced by phone browsing (type 91)
            Notification::QueueLoadTracks(q) => {
                info!(">>> NOTIF QueueLoadTracks ({} tracks, queue_pos={:?})", q.tracks.len(), q.queue_position);
                for (i, t) in q.tracks.iter().take(5).enumerate() {
                    info!("    track[{i}]: queue_item_id={:?} track_id={:?}", t.queue_item_id, t.track_id);
                }
                connect.queue = q.tracks;

                // If the server just assigned queue_item_ids for our locally-playing track
                // (response to LocalTrackStarted → push_queue), capture the id so heartbeats
                // report the correct track to the mobile app.
                if let Some(first) = connect.queue.first() {
                    if first.track_id == connect.current_track_id
                        && first.queue_item_id.is_some()
                        && first.queue_item_id.map(|id| id as i32) != connect.current_queue_item_id
                    {
                        let new_qid = first.queue_item_id.map(|id| id as i32);
                        info!("Connect: server assigned queue_item_id={new_qid:?} for local track");
                        connect.current_queue_item_id = new_qid;
                        return;
                    }
                }

                // Protocol convention: when the mobile browses and clicks a track, it appears
                // at index 0 with queue_item_id=None (not yet server-assigned).
                // If the first track has a queue_item_id, this is a server-state sync (e.g.
                // initial connect or session restore) — do NOT auto-play.
                let first = connect.queue.first();
                let is_fresh_selection = first.map_or(false, |t| t.queue_item_id.is_none());
                let selected_track_id = first.and_then(|t| t.track_id);
                if is_fresh_selection {
                    if let Some(track_id) = selected_track_id {
                        let is_same_track = Some(track_id) == connect.current_track_id;
                        if !is_same_track {
                            info!("Connect: QueueLoadTracks → new track selected: {track_id}, starting playback");
                        } else {
                            // User explicitly re-selected the same track (e.g. tapped it again on phone).
                            // Player was paused by the preceding SetState(Paused); restart from scratch.
                            info!("Connect: QueueLoadTracks → same track re-selected ({track_id}), restarting");
                        }
                        connect.current_track_id = Some(track_id);
                        connect.current_queue_item_id = None;
                        connect.playing = PlayingState::Playing;
                        connect.pending_play = None;
                        let state_clone = state.clone();
                        tokio::spawn(async move {
                            if let Err(e) = fetch_and_play(state_clone, track_id as i64, None).await {
                                error!("Connect: QueueLoadTracks fetch_and_play failed: {e}");
                            }
                        });
                        return; // skip pending_play resolution — new play supersedes it
                    }
                }

                // Resolve any pending play deferred from a SetState that arrived before the queue
                if let Some((pending_qid, pending_pos)) = connect.pending_play.take() {
                    if let Some(item) = connect.queue.iter().find(|t| t.queue_item_id == Some(pending_qid)) {
                        if let Some(track_id) = item.track_id {
                            info!("Connect: resolving deferred play for queue_item_id={pending_qid} → track_id={track_id}");
                            connect.current_track_id = Some(track_id);
                            connect.current_queue_item_id = Some(pending_qid as i32);
                            connect.playing = PlayingState::Playing;
                            let state_clone = state.clone();
                            tokio::spawn(async move {
                                if let Err(e) = fetch_and_play(state_clone, track_id as i64, pending_pos).await {
                                    error!("Connect: deferred fetch_and_play failed: {e}");
                                }
                            });
                        }
                    }
                }
            }

            // Queue cleared
            Notification::QueueCleared(_) => {
                info!(">>> NOTIF QueueCleared — clearing {} cached tracks", connect.queue.len());
                connect.queue.clear();
            }

            // Tracks inserted at a position (e.g. "play next")
            Notification::QueueTracksInserted(q) => {
                info!(">>> NOTIF QueueTracksInserted — {} new tracks, insert_after={:?}", q.tracks.len(), q.insert_after);
                let insert_after = q.insert_after.unwrap_or(-1);
                if insert_after < 0 {
                    // Prepend
                    let mut new_tracks = q.tracks;
                    new_tracks.extend(connect.queue.drain(..));
                    connect.queue = new_tracks;
                } else {
                    // Find insert position by queue_item_id
                    let pos = connect.queue.iter()
                        .position(|t| t.queue_item_id == Some(insert_after as u64))
                        .map(|p| p + 1)
                        .unwrap_or(connect.queue.len());
                    let tail = connect.queue.split_off(pos);
                    connect.queue.extend(q.tracks);
                    connect.queue.extend(tail);
                }
            }

            // Tracks appended
            Notification::QueueTracksAdded(q) => {
                info!(">>> NOTIF QueueTracksAdded — {} tracks appended (queue was {})", q.tracks.len(), connect.queue.len());
                connect.queue.extend(q.tracks);
            }

            Notification::AddRenderer(r) => {
                info!(">>> NOTIF AddRenderer id={:?} name={:?}", r.renderer_id, r.renderer.as_ref().and_then(|i| i.friendly_name.as_deref()));
                if let (Some(id), Some(info)) = (r.renderer_id, r.renderer) {
                    let name = info.friendly_name.unwrap_or_else(|| "Unknown".into());
                    let model = info.model.unwrap_or_default();
                    let mut list = state.connect_renderers.write();
                    // Upgrade existing mDNS entry (hash-based ID) for same device name
                    // so ActiveRendererChanged (which uses Qobuz numeric IDs) can mark it active.
                    if let Some(existing) = list.iter_mut().find(|x| x.name == name) {
                        existing.renderer_id = id;
                        if !model.is_empty() {
                            existing.model = model;
                        }
                    } else if !list.iter().any(|x| x.renderer_id == id) {
                        list.push(ConnectRenderer { renderer_id: id, name, model, is_active: false });
                    }
                }
            }

            Notification::UpdateRenderer(r) => {
                info!(">>> NOTIF UpdateRenderer name={:?}", r.renderer.as_ref().and_then(|i| i.friendly_name.as_deref()));
                if let Some(info) = r.renderer {
                    if let Some(name) = info.friendly_name {
                        let model = info.model.unwrap_or_default();
                        let mut list = state.connect_renderers.write();
                        if let Some(entry) = list.iter_mut().find(|x| x.name == name) {
                            entry.model = model;
                        }
                    }
                }
            }

            Notification::RemoveRenderer(r) => {
                info!(">>> NOTIF RemoveRenderer id={:?}", r.renderer_id);
                if let Some(id) = r.renderer_id {
                    state.connect_renderers.write().retain(|x| x.renderer_id != id);
                }
            }

            Notification::ActiveRendererChanged(r) => {
                info!(">>> NOTIF ActiveRendererChanged id={:?}", r.renderer_id);
                if let Some(active_id) = r.renderer_id {
                    let mut list = state.connect_renderers.write();
                    for entry in list.iter_mut() {
                        entry.is_active = entry.renderer_id == active_id;
                    }
                }
            }

            Notification::Deactivated => {
                info!(
                    ">>> NOTIF Deactivated — pausing (track_id={:?} pos={}ms) — session stays open for cast-back",
                    connect.current_track_id,
                    connect.position_ms,
                );
                state.player.write().pause();
                connect.playing = PlayingState::Paused;
            }

            Notification::VolumeChanged(v) => {
                let own_id = *state.connect_own_renderer_id.read();
                // Only apply if targeted at Q-Stream (own renderer_id).
                // renderer_id=None = broadcast; own_id=None = not yet registered (apply anyway).
                let for_us = own_id.map_or(true, |id| v.renderer_id == Some(id));
                if for_us {
                    if let Some(vol) = v.volume {
                        let vol_f = (vol as f32) / 100.0;
                        info!(">>> NOTIF VolumeChanged vol={vol} ({vol_f:.2})");
                        state.player.write().set_volume(vol_f);
                        let mut cfg = state.config.lock();
                        cfg.volume = vol_f;
                        crate::config::save(&cfg);
                    }
                }
            }

            Notification::VolumeMuted(m) => {
                let own_id = *state.connect_own_renderer_id.read();
                let for_us = own_id.map_or(true, |id| m.renderer_id == Some(id));
                if for_us {
                    info!(">>> NOTIF VolumeMuted muted={:?}", m.value);
                    if m.value == Some(true) {
                        state.player.write().set_volume(0.0);
                    }
                    // Unmute: server sends VolumeChanged with restored level right after.
                }
            }

            Notification::Connected => info!(">>> NOTIF Connected (WebSocket established)"),

            Notification::Disconnected { reason, .. } => {
                info!(">>> NOTIF Disconnected reason={:?}", reason);
            }

            Notification::DeviceRegistered { renderer_id, .. } => {
                info!(">>> NOTIF DeviceRegistered renderer_id={renderer_id}");
                *state.connect_own_renderer_id.write() = Some(renderer_id);
            }

            // State updates from the active renderer while we're inactive (after cast).
            // Updates remote playback state visible to the frontend, and tracks
            // which queue_item_id the remote renderer is on for next/prev navigation.
            Notification::RestoreState(rsu) => {
                if let Some(state_msg) = &rsu.state {
                    let is_playing = state_msg.playing_state
                        .and_then(|s| PlayingState::try_from(s).ok())
                        .map_or(false, |s| s == PlayingState::Playing);
                    let position_ms = state_msg.current_position
                        .as_ref()
                        .and_then(|p| p.value)
                        .unwrap_or(0) as u64;
                    let duration_ms = state_msg.duration.unwrap_or(0) as u64;
                    let now_ms = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis() as u64;

                    // Update or create remote state entry
                    {
                        let mut remote = state.connect_remote_state.write();
                        match remote.as_mut() {
                            Some(r) => {
                                r.is_playing = is_playing;
                                r.position_ms = position_ms;
                                r.duration_ms = duration_ms;
                                r.last_updated_at_ms = now_ms;
                            }
                            None => {
                                *remote = Some(ConnectRemoteState {
                                    is_playing,
                                    position_ms,
                                    duration_ms,
                                    last_updated_at_ms: now_ms,
                                    track: None,
                                });
                            }
                        }
                    }

                    // Track navigation (queue_item_id for next/prev) + metadata fetch
                    if let Some(idx) = state_msg.current_queue_index {
                        let track_ref = connect.queue.get(idx as usize);
                        let new_queue_item_id = track_ref
                            .and_then(|t| t.queue_item_id)
                            .map(|id| id as i32);
                        let new_track_id = track_ref.and_then(|t| t.track_id);

                        if new_queue_item_id != connect.current_queue_item_id {
                            info!(
                                "Connect: remote renderer moved to queue index {} → queue_item_id={:?}",
                                idx, new_queue_item_id
                            );
                            connect.current_queue_item_id = new_queue_item_id;
                        }

                        // Fetch track metadata when track changes
                        if new_track_id != connect.last_remote_track_id {
                            connect.last_remote_track_id = new_track_id;
                            if let Some(tid) = new_track_id {
                                info!("Connect: remote track changed to track_id={tid}, fetching metadata");
                                let state_clone = state.clone();
                                tokio::spawn(async move {
                                    if let Some(unified) = fetch_track_as_unified(&state_clone, tid as i64).await {
                                        let mut remote = state_clone.connect_remote_state.write();
                                        if let Some(r) = remote.as_mut() {
                                            r.track = Some(unified);
                                        }
                                    }
                                });
                            }
                        }
                    }
                }
            }

            other => {
                info!(">>> NOTIF (unhandled) {:?}", other);
            }
        },
    }
}

// ── Controller commands ──────────────────────────────────────────────────────

/// Handle a controller command: translate it to a `CtrlSrvrSetPlayerState` and send.
async fn handle_ctrl_cmd(
    session: &crate::qonductor::DeviceSession,
    connect: &mut ConnectState,
    cmd: ConnectCtrlCmd,
) {
    use crate::qonductor::PlayingState;
    let playing = PlayingState::Playing as i32;
    let paused  = PlayingState::Paused  as i32;

    let (ps, pos, qid): (Option<i32>, Option<u32>, Option<u32>) = match cmd {
        ConnectCtrlCmd::Play  => (Some(playing), None, None),
        ConnectCtrlCmd::Pause => (Some(paused),  None, None),
        ConnectCtrlCmd::Seek(ms) => (None, Some(ms), None),
        ConnectCtrlCmd::Next => {
            let next_id = adjacent_queue_item_id(&connect.queue, connect.current_queue_item_id, 1);
            if let Some(id) = next_id {
                connect.current_queue_item_id = Some(id as i32);
            }
            (Some(playing), Some(0), next_id)
        }
        ConnectCtrlCmd::Prev => {
            let prev_id = adjacent_queue_item_id(&connect.queue, connect.current_queue_item_id, -1);
            // prev_id=None means we're going to track[0] (queue_item_id=None)
            connect.current_queue_item_id = prev_id.map(|id| id as i32);
            (Some(playing), Some(0), prev_id)
        }
        // Handled before reaching this function; included for exhaustiveness
        ConnectCtrlCmd::LocalTrackStarted(_)
        | ConnectCtrlCmd::LocalPaused
        | ConnectCtrlCmd::LocalResumed
        | ConnectCtrlCmd::LocalSeeked(_) => return,
    };

    info!("Connect ctrl: sending CtrlSrvrSetPlayerState ps={ps:?} pos={pos:?} qid={qid:?}");
    if let Err(e) = session.control_player(ps, pos, qid).await {
        warn!("Connect ctrl: failed to send player state: {e}");
    }
}

/// Find the queue_item_id `delta` steps away from the current track.
/// Returns `None` for the first track (queue_item_id=None in proto convention).
fn adjacent_queue_item_id(
    queue: &[msg::QueueTrackRef],
    current_id: Option<i32>,
    delta: i32,
) -> Option<u32> {
    // Find position of current track in the queue
    let current_pos = if let Some(id) = current_id {
        queue.iter().position(|t| t.queue_item_id == Some(id as u64)).unwrap_or(0)
    } else {
        0 // current_id=None → first track (index 0)
    };
    let target = current_pos as i32 + delta;
    if target < 0 { return None; }
    queue.get(target as usize)?.queue_item_id.map(|id| id as u32)
}

// ── Track fetching ──────────────────────────────────────────────────────────

/// Fetch Qobuz track metadata and return it as a UnifiedTrack (no audio bytes, no playback).
async fn fetch_track_as_unified(state: &Arc<AppState>, track_id: i64) -> Option<UnifiedTrack> {
    let qobuz = state.qobuz.read().clone()?;
    let track = qobuz.get_track(track_id).await.ok()?;
    let cover_url = track
        .album
        .as_ref()
        .and_then(|a| a.image.as_ref())
        .and_then(|img| img.large.clone().or_else(|| img.small.clone()));
    let artist = track
        .performer
        .as_ref()
        .map(|p| p.name.clone())
        .or_else(|| track.album.as_ref().and_then(|a| a.artist.as_ref()).map(|a| a.name.clone()))
        .unwrap_or_else(|| "Unknown".to_string());
    let album_title = track.album.as_ref().map(|a| a.title.clone()).unwrap_or_default();
    Some(UnifiedTrack {
        id: track_id.to_string(),
        title: track.title,
        artist,
        album: album_title,
        duration_seconds: track.duration,
        cover_url,
        source: TrackSource::Qobuz { track_id },
        quality_label: None,
        sample_rate: track.maximum_sampling_rate,
        bit_depth: track.maximum_bit_depth,
    })
}

async fn fetch_and_play(
    state: Arc<AppState>,
    track_id: i64,
    seek_to_ms: Option<u32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let qobuz = state.qobuz.read().clone();
    let qobuz = qobuz.ok_or("Not logged in to Qobuz")?;

    // Fetch track metadata
    let track = qobuz.get_track(track_id).await?;

    let cover_url = track
        .album
        .as_ref()
        .and_then(|a| a.image.as_ref())
        .and_then(|img| img.large.clone().or_else(|| img.small.clone()));

    let artist = track
        .performer
        .as_ref()
        .map(|p| p.name.clone())
        .or_else(|| {
            track
                .album
                .as_ref()
                .and_then(|a| a.artist.as_ref())
                .map(|a| a.name.clone())
        })
        .unwrap_or_else(|| "Unknown".to_string());

    let album_title = track
        .album
        .as_ref()
        .map(|a| a.title.clone())
        .unwrap_or_default();

    let sample_rate = track.maximum_sampling_rate;
    let bit_depth = track.maximum_bit_depth;

    let unified = UnifiedTrack {
        id: track_id.to_string(),
        title: track.title.clone(),
        artist,
        album: album_title,
        duration_seconds: track.duration,
        cover_url,
        source: TrackSource::Qobuz { track_id },
        quality_label: None,
        sample_rate,
        bit_depth,
    };

    // Fetch audio bytes (disk cache or network)
    let track_url = qobuz.get_track_url(track_id).await?;
    let bytes = qobuz.fetch_track_bytes(&track_url).await?;

    // Play
    {
        let mut player = state.player.write();
        player.play_bytes(bytes, unified, sample_rate, bit_depth)?;
    }

    // Seek if the app requested a specific position
    if let Some(pos) = seek_to_ms {
        if pos > 0 {
            let _ = state.player.write().seek(pos as u64);
        }
    }

    Ok(())
}
