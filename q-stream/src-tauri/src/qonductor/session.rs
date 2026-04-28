use std::sync::Arc;

use tokio::sync::{RwLock, mpsc};

use super::event::SessionEvent;
use super::msg::QueueRendererState;

#[derive(Debug, Clone)]
pub enum SessionCommand {
    ReportState(QueueRendererState),
    ReportVolume(u32),
    ReportVolumeMuted(bool),
    ReportMaxAudioQuality(i32),
    ReportFileAudioQuality(u32),
    SetActiveRenderer(u64),
    ControlPlayer {
        playing_state: Option<i32>,
        position_ms: Option<u32>,
        queue_item_id: Option<u32>,
    },
    PushQueue(Vec<u32>),
}

pub(crate) type SharedCommandTx = Arc<RwLock<Option<mpsc::Sender<SessionCommand>>>>;

pub struct DeviceSession {
    events: mpsc::Receiver<SessionEvent>,
    command_tx: SharedCommandTx,
}

impl DeviceSession {
    pub(crate) fn new(events: mpsc::Receiver<SessionEvent>, command_tx: SharedCommandTx) -> Self {
        Self { events, command_tx }
    }

    pub async fn recv(&mut self) -> Option<SessionEvent> {
        self.events.recv().await
    }

    async fn send_command(&self, cmd: SessionCommand) -> super::Result<()> {
        let guard = self.command_tx.read().await;
        match &*guard {
            Some(tx) => tx
                .send(cmd)
                .await
                .map_err(|_| super::Error::Session("Session closed".to_string())),
            None => Err(super::Error::Session("Not connected".to_string())),
        }
    }

    pub async fn report_state(&self, state: QueueRendererState) -> super::Result<()> {
        self.send_command(SessionCommand::ReportState(state)).await
    }

    pub async fn report_volume(&self, volume: u32) -> super::Result<()> {
        self.send_command(SessionCommand::ReportVolume(volume)).await
    }

    pub async fn report_muted(&self, muted: bool) -> super::Result<()> {
        self.send_command(SessionCommand::ReportVolumeMuted(muted)).await
    }

    pub async fn report_max_audio_quality(&self, quality: i32) -> super::Result<()> {
        self.send_command(SessionCommand::ReportMaxAudioQuality(quality)).await
    }

    pub async fn report_file_audio_quality(&self, sample_rate_hz: u32) -> super::Result<()> {
        self.send_command(SessionCommand::ReportFileAudioQuality(sample_rate_hz)).await
    }

    pub async fn cast_to(&self, renderer_id: u64) -> super::Result<()> {
        self.send_command(SessionCommand::SetActiveRenderer(renderer_id)).await
    }

    pub async fn control_player(
        &self,
        playing_state: Option<i32>,
        position_ms: Option<u32>,
        queue_item_id: Option<u32>,
    ) -> super::Result<()> {
        self.send_command(SessionCommand::ControlPlayer { playing_state, position_ms, queue_item_id })
            .await
    }

    pub async fn push_queue(&self, track_ids: Vec<u32>) -> super::Result<()> {
        self.send_command(SessionCommand::PushQueue(track_ids)).await
    }
}
