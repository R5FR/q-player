use std::pin::Pin;

use futures::StreamExt;
use futures::stream::Stream;
use tokio::sync::{mpsc, oneshot};
use tokio::time::{Duration, interval};
use tracing::{debug, info, warn};

use super::Result;
use super::config::{DeviceConfig, SessionInfo};
use super::connection::{Connection, ConnectionWriter};
use super::event::{
    dispatch_notification, ActivationState, Command, Notification, Responder, SessionEvent,
};
use super::msg::{
    ctrl::{AskForQueueState, AskForRendererState, SetActiveRenderer},
    report::{
        FileAudioQualityChanged, MaxAudioQualityChanged, StateUpdated, VolumeChanged, VolumeMuted,
    },
    QueueRendererState,
};
use super::proto::qconnect::{QConnectMessage, QConnectMessageType};
use super::session::SessionCommand;

pub async fn spawn_session(
    session_info: &SessionInfo,
    device_config: &DeviceConfig,
    event_tx: mpsc::Sender<SessionEvent>,
    command_rx: mpsc::Receiver<SessionCommand>,
) -> Result<()> {
    debug!(
        session_id = %session_info.session_id,
        device = %device_config.friendly_name,
        "Connecting session"
    );

    let mut connection =
        Connection::connect(&session_info.ws_endpoint, &session_info.ws_jwt).await?;
    connection.subscribe_default().await?;
    connection
        .join_session(&device_config.device_uuid, &device_config.friendly_name)
        .await?;

    let (reader, writer) = connection.split();
    let reader = Box::pin(reader.into_stream());

    let _ = event_tx
        .send(SessionEvent::Notification(Notification::Connected))
        .await;

    let runner = SessionRunner {
        session_id: session_info.session_id.clone(),
        reader,
        writer,
        device_uuid: device_config.device_uuid,
        device_name: device_config.friendly_name.clone(),
        renderer_id: 0,
        is_active: false,
        event_tx,
        command_rx,
        state: SessionState::default(),
        api_jwt: session_info.api_jwt.clone(),
    };

    tokio::spawn(async move {
        runner.run().await;
    });

    Ok(())
}

#[derive(Default)]
struct SessionState {
    #[allow(dead_code)]
    session_id: Option<u64>,
    session_uuid: Option<[u8; 16]>,
    queue_version: Option<super::proto::qconnect::QueueVersion>,
}

const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(10);

struct SessionRunner {
    session_id: String,
    reader: Pin<Box<dyn Stream<Item = Result<QConnectMessage>> + Send>>,
    writer: ConnectionWriter,
    device_uuid: [u8; 16],
    device_name: String,
    renderer_id: u64,
    is_active: bool,
    event_tx: mpsc::Sender<SessionEvent>,
    command_rx: mpsc::Receiver<SessionCommand>,
    state: SessionState,
    api_jwt: String,
}

impl SessionRunner {
    async fn run(mut self) {
        info!(session_id = %self.session_id, "Session runner starting");

        let mut heartbeat = interval(HEARTBEAT_INTERVAL);
        heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                msg = self.reader.next() => {
                    match msg {
                        Some(Ok(m)) => {
                            match self.handle_qconnect_message(m).await {
                                Ok(true) => break,
                                Ok(false) => {}
                                Err(e) => warn!(error = %e, "Error handling message"),
                            }
                        }
                        Some(Err(e)) => {
                            warn!(error = %e, "WebSocket error");
                            let _ = self.event_tx.send(SessionEvent::Notification(Notification::Disconnected {
                                session_id: self.session_id.clone(),
                                reason: Some(e.to_string()),
                            })).await;
                            break;
                        }
                        None => {
                            info!("WebSocket closed");
                            let _ = self.event_tx.send(SessionEvent::Notification(Notification::Disconnected {
                                session_id: self.session_id.clone(),
                                reason: None,
                            })).await;
                            break;
                        }
                    }
                }

                _ = heartbeat.tick() => {
                    if self.is_active && self.renderer_id != 0 {
                        let (tx, rx) = oneshot::channel();
                        let _ = self.event_tx.send(SessionEvent::Command(Command::Heartbeat {
                            respond: Responder::new(tx),
                        })).await;
                        if let Ok(Some(resp)) = rx.await {
                            if let Err(e) = self.send_renderer_state(&resp).await {
                                warn!(error = %e, "Failed to send heartbeat");
                            }
                        }
                    }
                }

                cmd = self.command_rx.recv() => {
                    match cmd {
                        Some(command) => {
                            if let Err(e) = self.handle_command(command).await {
                                warn!(error = %e, "Failed to handle command");
                            }
                        }
                        None => {
                            debug!("Command channel closed");
                        }
                    }
                }
            }
        }

        let _ = self
            .event_tx
            .send(SessionEvent::Notification(Notification::SessionClosed {
                device_uuid: self.device_uuid,
            }))
            .await;
        info!(session_id = %self.session_id, "Session runner stopped");
    }

    async fn handle_command(&mut self, command: SessionCommand) -> Result<()> {
        match command {
            SessionCommand::ReportState(resp) => self.send_renderer_state(&resp).await,
            SessionCommand::ReportVolume(volume) => self.do_report_volume(volume).await,
            SessionCommand::ReportVolumeMuted(muted) => self.do_report_volume_muted(muted).await,
            SessionCommand::ReportMaxAudioQuality(quality) => {
                self.do_report_max_audio_quality(quality).await
            }
            SessionCommand::ReportFileAudioQuality(sample_rate_hz) => {
                self.do_report_file_audio_quality(sample_rate_hz).await
            }
            SessionCommand::SetActiveRenderer(renderer_id) => {
                self.do_set_active_renderer(renderer_id).await
            }
            SessionCommand::ControlPlayer { playing_state, position_ms, queue_item_id } => {
                self.do_control_player(playing_state, position_ms, queue_item_id).await
            }
            SessionCommand::PushQueue(track_ids) => {
                self.do_push_queue(track_ids).await
            }
        }
    }

    async fn do_control_player(
        &mut self,
        playing_state: Option<i32>,
        position_ms: Option<u32>,
        queue_item_id: Option<u32>,
    ) -> Result<()> {
        use super::proto::qconnect::{CtrlSrvrSetPlayerState, QueueItemRef};
        let current_queue_item = queue_item_id.map(|id| QueueItemRef {
            queue_version: None,
            id: Some(id),
        });
        let msg = super::proto::qconnect::QConnectMessage {
            message_type: Some(
                super::proto::qconnect::QConnectMessageType::MessageTypeCtrlSrvrSetPlayerState as i32,
            ),
            ctrl_srvr_set_player_state: Some(CtrlSrvrSetPlayerState {
                playing_state,
                current_position: position_ms,
                current_queue_item,
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_push_queue(&mut self, track_ids: Vec<u32>) -> Result<()> {
        use super::proto::qconnect::{
            CtrlSrvrClearQueue, CtrlSrvrQueueInsertTracks, QueueTrackRef,
            QConnectMessageType,
        };

        let clear = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeCtrlSrvrClearQueue as i32),
            ctrl_srvr_clear_queue: Some(CtrlSrvrClearQueue {
                queue_version: self.state.queue_version,
            }),
            ..Default::default()
        };
        self.writer.send(clear).await?;

        let tracks = track_ids
            .into_iter()
            .map(|id| QueueTrackRef { queue_item_id: None, track_id: Some(id), context_uuid: None })
            .collect();
        let insert = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeCtrlSrvrQueueInsertTracks as i32),
            ctrl_srvr_queue_insert_tracks: Some(CtrlSrvrQueueInsertTracks {
                tracks,
                insert_after: Some(-1),
                ..Default::default()
            }),
            ..Default::default()
        };
        self.writer.send(insert).await
    }

    async fn do_set_active_renderer(&mut self, renderer_id: u64) -> Result<()> {
        let msg = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeCtrlSrvrSetActiveRenderer as i32),
            ctrl_srvr_set_active_renderer: Some(SetActiveRenderer {
                renderer_id: Some(renderer_id as i32),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_report_volume(&mut self, volume: u32) -> Result<()> {
        let msg = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeRndrSrvrVolumeChanged as i32),
            rndr_srvr_volume_changed: Some(VolumeChanged {
                volume: Some(volume),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_report_volume_muted(&mut self, muted: bool) -> Result<()> {
        let msg = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeRndrSrvrVolumeMuted as i32),
            rndr_srvr_volume_muted: Some(VolumeMuted {
                value: if muted { Some(true) } else { None },
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_report_max_audio_quality(&mut self, quality: i32) -> Result<()> {
        let msg = QConnectMessage {
            message_type: Some(
                QConnectMessageType::MessageTypeRndrSrvrMaxAudioQualityChanged as i32,
            ),
            rndr_srvr_max_audio_quality_changed: Some(MaxAudioQualityChanged {
                value: Some(quality),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_report_file_audio_quality(&mut self, sample_rate_hz: u32) -> Result<()> {
        let msg = QConnectMessage {
            message_type: Some(
                QConnectMessageType::MessageTypeRndrSrvrFileAudioQualityChanged as i32,
            ),
            rndr_srvr_file_audio_quality_changed: Some(FileAudioQualityChanged {
                value: Some(sample_rate_hz as i32),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_request_queue_state(&mut self) -> Result<()> {
        let queue_uuid = self.state.session_uuid.unwrap_or(self.device_uuid);
        let msg = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeCtrlSrvrAskForQueueState as i32),
            ctrl_srvr_ask_for_queue_state: Some(AskForQueueState {
                queue_version: None,
                queue_uuid: Some(queue_uuid.to_vec()),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn do_request_renderer_state(&mut self) -> Result<()> {
        let session_id = self.state.session_id.unwrap_or(0);
        let msg = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeCtrlSrvrAskForRendererState as i32),
            ctrl_srvr_ask_for_renderer_state: Some(AskForRendererState {
                session_id: Some(session_id),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn send_renderer_state(&mut self, state: &QueueRendererState) -> Result<()> {
        let msg = QConnectMessage {
            message_type: Some(QConnectMessageType::MessageTypeRndrSrvrStateUpdated as i32),
            rndr_srvr_state_updated: Some(StateUpdated {
                state: Some(*state),
            }),
            ..Default::default()
        };
        self.writer.send(msg).await
    }

    async fn send_activation_handshake(&mut self, state: &ActivationState) -> Result<()> {
        self.do_report_volume_muted(state.muted).await?;
        self.do_report_volume(state.volume).await?;
        self.do_report_max_audio_quality(state.max_quality).await?;
        Ok(())
    }

    async fn handle_qconnect_message(&mut self, mut msg: QConnectMessage) -> Result<bool> {
        let msg_type = msg.message_type.unwrap_or(0);
        debug!(
            msg_type,
            session = %self.session_id,
            renderer_id = self.renderer_id,
            "WS ← raw message"
        );

        match msg_type {
            t if t == QConnectMessageType::MessageTypeSrvrCtrlAddRenderer as i32 => {
                if let Some(add) = &msg.srvr_ctrl_add_renderer {
                    if let Some(renderer) = &add.renderer {
                        let renderer_uuid: Option<[u8; 16]> = renderer
                            .device_uuid
                            .as_ref()
                            .and_then(|u| u.as_slice().try_into().ok());

                        let rid = add.renderer_id.unwrap_or(0);

                        if renderer_uuid == Some(self.device_uuid) {
                            self.renderer_id = rid;
                            info!(renderer_id = rid, name = %self.device_name, "Our device registered");

                            let _ = self
                                .event_tx
                                .send(SessionEvent::Notification(Notification::DeviceRegistered {
                                    device_uuid: self.device_uuid,
                                    renderer_id: rid,
                                    api_jwt: self.api_jwt.clone(),
                                }))
                                .await;

                            let set_active_msg = QConnectMessage {
                                message_type: Some(
                                    QConnectMessageType::MessageTypeCtrlSrvrSetActiveRenderer as i32,
                                ),
                                ctrl_srvr_set_active_renderer: Some(SetActiveRenderer {
                                    renderer_id: Some(rid as i32),
                                }),
                                ..Default::default()
                            };
                            self.writer.send(set_active_msg).await?;
                        }
                    }
                }
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self
                        .event_tx
                        .send(SessionEvent::Notification(notification))
                        .await;
                }
            }

            t if t == QConnectMessageType::MessageTypeSrvrCtrlRemoveRenderer as i32 => {
                if let Some(rem) = &msg.srvr_ctrl_remove_renderer {
                    let rid = rem.renderer_id.unwrap_or(0);
                    if self.renderer_id == rid {
                        self.renderer_id = 0;
                    }
                }
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self
                        .event_tx
                        .send(SessionEvent::Notification(notification))
                        .await;
                }
            }

            t if t == QConnectMessageType::MessageTypeSrvrCtrlSessionState as i32 => {
                if let Some(ss) = &msg.srvr_ctrl_session_state {
                    self.state.session_id = Some(ss.session_id.unwrap_or(0));

                    if let Some(uuid_bytes) = &ss.session_uuid {
                        if uuid_bytes.len() == 16 {
                            let mut uuid = [0u8; 16];
                            uuid.copy_from_slice(uuid_bytes);
                            self.state.session_uuid = Some(uuid);
                        }
                    }

                    if let Err(e) = self.do_request_renderer_state().await {
                        warn!(error = %e, "Failed to request renderer state");
                    }
                }
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self
                        .event_tx
                        .send(SessionEvent::Notification(notification))
                        .await;
                }
            }

            t if t == QConnectMessageType::MessageTypeSrvrCtrlRendererStateUpdated as i32 => {
                if let Some(rsu) = msg.srvr_ctrl_renderer_state_updated {
                    let rid = rsu.renderer_id.unwrap_or(0);

                    if rid != self.renderer_id && !self.is_active {
                        let _ = self
                            .event_tx
                            .send(SessionEvent::Notification(Notification::RestoreState(rsu)))
                            .await;
                    } else {
                        let _ = self
                            .event_tx
                            .send(SessionEvent::Notification(Notification::RendererStateUpdated(
                                rsu,
                            )))
                            .await;
                    }
                }
            }

            t if t == QConnectMessageType::MessageTypeSrvrRndrSetState as i32 => {
                if let Some(ss) = msg.srvr_rndr_set_state {
                    if ss.playing_state.is_some() || ss.current_position.is_some() || ss.current_queue_item.is_some() {
                        let (tx, rx) = oneshot::channel();
                        let _ = self
                            .event_tx
                            .send(SessionEvent::Command(Command::SetState {
                                cmd: ss,
                                respond: Responder::new(tx),
                            }))
                            .await;
                        if let Ok(response) = rx.await {
                            if let Err(e) = self.send_renderer_state(&response).await {
                                warn!(error = %e, "Failed to send playback response");
                            }
                        }
                    }
                }
            }

            t if t == QConnectMessageType::MessageTypeSrvrRndrSetActive as i32 => {
                if let Some(sa) = msg.srvr_rndr_set_active {
                    let active = sa.active.unwrap_or(false);

                    if active {
                        info!(renderer_id = self.renderer_id, "Server set us active");
                        self.is_active = true;

                        let (tx, rx) = oneshot::channel();
                        let _ = self
                            .event_tx
                            .send(SessionEvent::Command(Command::SetActive {
                                cmd: sa,
                                respond: Responder::new(tx),
                            }))
                            .await;
                        if let Ok(activation_state) = rx.await {
                            if let Err(e) = self.send_activation_handshake(&activation_state).await {
                                warn!(error = %e, "Failed to send activation handshake");
                            }
                        }

                        if let Err(e) = self.do_request_queue_state().await {
                            warn!(error = %e, "Failed to request queue state");
                        }
                    } else {
                        info!(renderer_id = self.renderer_id, "Server set us inactive");
                        self.is_active = false;

                        let _ = self
                            .event_tx
                            .send(SessionEvent::Notification(Notification::Deactivated))
                            .await;
                    }
                }
            }

            t if t == QConnectMessageType::MessageTypeSrvrCtrlQueueState as i32 => {
                if let Some(qs) = &msg.srvr_ctrl_queue_state {
                    if qs.queue_version.is_some() {
                        self.state.queue_version = qs.queue_version;
                    }
                }
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self.event_tx.send(SessionEvent::Notification(notification)).await;
                }
            }
            t if t == QConnectMessageType::MessageTypeSrvrCtrlQueueVersionChanged as i32 => {
                if let Some(qvc) = &msg.srvr_ctrl_queue_version_changed {
                    if qvc.queue_version.is_some() {
                        self.state.queue_version = qvc.queue_version;
                    }
                }
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self.event_tx.send(SessionEvent::Notification(notification)).await;
                }
            }
            t if t == QConnectMessageType::MessageTypeSrvrCtrlQueueCleared as i32 => {
                if let Some(qc) = &msg.srvr_ctrl_queue_cleared {
                    if qc.queue_version.is_some() {
                        self.state.queue_version = qc.queue_version;
                    }
                }
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self.event_tx.send(SessionEvent::Notification(notification)).await;
                }
            }

            _ => {
                if let Some(notification) = dispatch_notification(&mut msg) {
                    let _ = self
                        .event_tx
                        .send(SessionEvent::Notification(notification))
                        .await;
                }
            }
        }

        Ok(false)
    }
}
