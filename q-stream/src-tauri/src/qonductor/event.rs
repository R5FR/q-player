use tokio::sync::oneshot;

use super::msg::{self, QueueRendererState};
use super::proto::qconnect::{QConnectMessage, QConnectMessageType};

#[derive(Debug, Clone)]
pub struct ActivationState {
    pub muted: bool,
    pub volume: u32,
    pub max_quality: i32,
    pub playback: QueueRendererState,
}

#[must_use = "call .send() to respond or the session will hang"]
pub struct Responder<T> {
    tx: oneshot::Sender<T>,
}

impl<T> Responder<T> {
    pub(crate) fn new(tx: oneshot::Sender<T>) -> Self {
        Self { tx }
    }

    pub fn send(self, value: T) {
        let _ = self.tx.send(value);
    }
}

pub enum SessionEvent {
    Command(Command),
    Notification(Notification),
}

pub enum Command {
    SetState {
        cmd: msg::cmd::SetState,
        respond: Responder<QueueRendererState>,
    },
    SetActive {
        cmd: msg::cmd::SetActive,
        respond: Responder<ActivationState>,
    },
    Heartbeat {
        respond: Responder<Option<QueueRendererState>>,
    },
}

macro_rules! define_notifications {
    (
        $(
            $variant:ident, $field:ident, $msg_type:ident
        );* $(;)?
    ) => {
        #[derive(Debug)]
        #[non_exhaustive]
        pub enum Notification {
            $(
                $variant(msg::notify::$variant),
            )*
            Deactivated,
            RestoreState(msg::notify::RendererStateUpdated),
            Connected,
            Disconnected {
                session_id: String,
                reason: Option<String>,
            },
            DeviceRegistered {
                device_uuid: [u8; 16],
                renderer_id: u64,
                api_jwt: String,
            },
            SessionClosed { device_uuid: [u8; 16] },
        }

        pub(crate) fn dispatch_notification(msg: &mut QConnectMessage) -> Option<Notification> {
            let msg_type = msg.message_type?;
            $(
                if msg_type == QConnectMessageType::$msg_type as i32 {
                    return msg.$field.take().map(Notification::$variant);
                }
            )*
            None
        }
    };
}

define_notifications! {
    SessionState, srvr_ctrl_session_state, MessageTypeSrvrCtrlSessionState;
    QueueState, srvr_ctrl_queue_state, MessageTypeSrvrCtrlQueueState;
    QueueCleared, srvr_ctrl_queue_cleared, MessageTypeSrvrCtrlQueueCleared;
    QueueLoadTracks, srvr_ctrl_queue_tracks_loaded, MessageTypeSrvrCtrlQueueTracksLoaded;
    QueueTracksAdded, srvr_ctrl_queue_tracks_added, MessageTypeSrvrCtrlQueueTracksAdded;
    QueueTracksInserted, srvr_ctrl_queue_tracks_inserted, MessageTypeSrvrCtrlQueueTracksInserted;
    QueueTracksRemoved, srvr_ctrl_queue_tracks_removed, MessageTypeSrvrCtrlQueueTracksRemoved;
    QueueTracksReordered, srvr_ctrl_queue_tracks_reordered, MessageTypeSrvrCtrlQueueTracksReordered;
    QueueVersionChanged, srvr_ctrl_queue_version_changed, MessageTypeSrvrCtrlQueueVersionChanged;
    QueueErrorMessage, srvr_ctrl_queue_error_message, MessageTypeSrvrCtrlQueueErrorMessage;
    AutoplayModeSet, srvr_ctrl_autoplay_mode_set, MessageTypeSrvrCtrlAutoplayModeSet;
    AutoplayTracksLoaded, srvr_ctrl_autoplay_tracks_loaded, MessageTypeSrvrCtrlAutoplayTracksLoaded;
    LoopModeSet, srvr_ctrl_loop_mode_set, MessageTypeSrvrCtrlLoopModeSet;
    ShuffleModeSet, srvr_ctrl_shuffle_mode_set, MessageTypeSrvrCtrlShuffleModeSet;
    ActiveRendererChanged, srvr_ctrl_active_renderer_changed, MessageTypeSrvrCtrlActiveRendererChanged;
    AddRenderer, srvr_ctrl_add_renderer, MessageTypeSrvrCtrlAddRenderer;
    UpdateRenderer, srvr_ctrl_update_renderer, MessageTypeSrvrCtrlUpdateRenderer;
    RemoveRenderer, srvr_ctrl_remove_renderer, MessageTypeSrvrCtrlRemoveRenderer;
    RendererStateUpdated, srvr_ctrl_renderer_state_updated, MessageTypeSrvrCtrlRendererStateUpdated;
    VolumeChanged, srvr_ctrl_volume_changed, MessageTypeSrvrCtrlVolumeChanged;
    VolumeMuted, srvr_ctrl_volume_muted, MessageTypeSrvrCtrlVolumeMuted;
    MaxAudioQualityChanged, srvr_ctrl_max_audio_quality_changed, MessageTypeSrvrCtrlMaxAudioQualityChanged;
    FileAudioQualityChanged, srvr_ctrl_file_audio_quality_changed, MessageTypeSrvrCtrlFileAudioQualityChanged;
    DeviceAudioQualityChanged, srvr_ctrl_device_audio_quality_changed, MessageTypeSrvrCtrlDeviceAudioQualityChanged;
}
