pub mod cmd {
    pub use super::super::proto::qconnect::SrvrRndrSetActive as SetActive;
    pub use super::super::proto::qconnect::SrvrRndrSetAutoplayMode as SetAutoplayMode;
    pub use super::super::proto::qconnect::SrvrRndrSetLoopMode as SetLoopMode;
    pub use super::super::proto::qconnect::SrvrRndrSetMaxAudioQuality as SetMaxAudioQuality;
    pub use super::super::proto::qconnect::SrvrRndrSetShuffleMode as SetShuffleMode;
    pub use super::super::proto::qconnect::SrvrRndrSetState as SetState;
    pub use super::super::proto::qconnect::SrvrRndrSetVolume as SetVolume;
}

pub mod report {
    pub use super::super::proto::qconnect::RndrSrvrJoinSession as JoinSession;
    pub use super::super::proto::qconnect::RndrSrvrDeviceInfoUpdated as DeviceInfoUpdated;
    pub use super::super::proto::qconnect::RndrSrvrStateUpdated as StateUpdated;
    pub use super::super::proto::qconnect::RndrSrvrRendererAction as RendererAction;
    pub use super::super::proto::qconnect::RndrSrvrVolumeChanged as VolumeChanged;
    pub use super::super::proto::qconnect::RndrSrvrVolumeMuted as VolumeMuted;
    pub use super::super::proto::qconnect::RndrSrvrFileAudioQualityChanged as FileAudioQualityChanged;
    pub use super::super::proto::qconnect::RndrSrvrDeviceAudioQualityChanged as DeviceAudioQualityChanged;
    pub use super::super::proto::qconnect::RndrSrvrMaxAudioQualityChanged as MaxAudioQualityChanged;
}

pub mod ctrl {
    pub use super::super::proto::qconnect::CtrlSrvrJoinSession as JoinSession;
    pub use super::super::proto::qconnect::CtrlSrvrSetPlayerState as SetPlayerState;
    pub use super::super::proto::qconnect::CtrlSrvrSetActiveRenderer as SetActiveRenderer;
    pub use super::super::proto::qconnect::CtrlSrvrSetVolume as SetVolume;
    pub use super::super::proto::qconnect::CtrlSrvrMuteVolume as MuteVolume;
    pub use super::super::proto::qconnect::CtrlSrvrClearQueue as ClearQueue;
    pub use super::super::proto::qconnect::CtrlSrvrQueueLoadTracks as QueueLoadTracks;
    pub use super::super::proto::qconnect::CtrlSrvrQueueInsertTracks as QueueInsertTracks;
    pub use super::super::proto::qconnect::CtrlSrvrQueueAddTracks as QueueAddTracks;
    pub use super::super::proto::qconnect::CtrlSrvrQueueRemoveTracks as QueueRemoveTracks;
    pub use super::super::proto::qconnect::CtrlSrvrQueueReorderTracks as QueueReorderTracks;
    pub use super::super::proto::qconnect::CtrlSrvrSetQueueState as SetQueueState;
    pub use super::super::proto::qconnect::CtrlSrvrSetShuffleMode as SetShuffleMode;
    pub use super::super::proto::qconnect::CtrlSrvrSetLoopMode as SetLoopMode;
    pub use super::super::proto::qconnect::CtrlSrvrSetMaxAudioQuality as SetMaxAudioQuality;
    pub use super::super::proto::qconnect::CtrlSrvrSetAutoplayMode as SetAutoplayMode;
    pub use super::super::proto::qconnect::CtrlSrvrAskForQueueState as AskForQueueState;
    pub use super::super::proto::qconnect::CtrlSrvrAskForRendererState as AskForRendererState;
    pub use super::super::proto::qconnect::CtrlSrvrAutoplayLoadTracks as AutoplayLoadTracks;
    pub use super::super::proto::qconnect::CtrlSrvrAutoplayRemoveTracks as AutoplayRemoveTracks;
}

pub mod notify {
    pub use super::super::proto::qconnect::SrvrCtrlSessionState as SessionState;
    pub use super::super::proto::qconnect::SrvrCtrlQueueState as QueueState;
    pub use super::super::proto::qconnect::SrvrCtrlQueueCleared as QueueCleared;
    pub use super::super::proto::qconnect::SrvrCtrlQueueLoadTracks as QueueLoadTracks;
    pub use super::super::proto::qconnect::SrvrCtrlQueueTracksAdded as QueueTracksAdded;
    pub use super::super::proto::qconnect::SrvrCtrlQueueTracksInserted as QueueTracksInserted;
    pub use super::super::proto::qconnect::SrvrCtrlQueueTracksRemoved as QueueTracksRemoved;
    pub use super::super::proto::qconnect::SrvrCtrlQueueTracksReordered as QueueTracksReordered;
    pub use super::super::proto::qconnect::SrvrCtrlQueueVersionChanged as QueueVersionChanged;
    pub use super::super::proto::qconnect::SrvrCtrlQueueErrorMessage as QueueErrorMessage;
    pub use super::super::proto::qconnect::SrvrCtrlAutoplayModeSet as AutoplayModeSet;
    pub use super::super::proto::qconnect::SrvrCtrlAutoplayTracksLoaded as AutoplayTracksLoaded;
    pub use super::super::proto::qconnect::SrvrCtrlLoopModeSet as LoopModeSet;
    pub use super::super::proto::qconnect::SrvrCtrlShuffleModeSet as ShuffleModeSet;
    pub use super::super::proto::qconnect::SrvrCtrlActiveRendererChanged as ActiveRendererChanged;
    pub use super::super::proto::qconnect::SrvrCtrlAddRenderer as AddRenderer;
    pub use super::super::proto::qconnect::SrvrCtrlUpdateRenderer as UpdateRenderer;
    pub use super::super::proto::qconnect::SrvrCtrlRemoveRenderer as RemoveRenderer;
    pub use super::super::proto::qconnect::SrvrCtrlRendererStateUpdated as RendererStateUpdated;
    pub use super::super::proto::qconnect::SrvrCtrlVolumeChanged as VolumeChanged;
    pub use super::super::proto::qconnect::SrvrCtrlVolumeMuted as VolumeMuted;
    pub use super::super::proto::qconnect::SrvrCtrlMaxAudioQualityChanged as MaxAudioQualityChanged;
    pub use super::super::proto::qconnect::SrvrCtrlFileAudioQualityChanged as FileAudioQualityChanged;
    pub use super::super::proto::qconnect::SrvrCtrlDeviceAudioQualityChanged as DeviceAudioQualityChanged;
}

pub use super::proto::qconnect::{
    BufferState, DeviceType, LoopMode, PlayingState,
    DeviceCapabilities, DeviceInfo, Position, QueueItemRef, QueueRendererState, QueueTrackRef,
    QueueVersion, RendererState,
};

pub trait QueueRendererStateExt {
    fn state(&self) -> Option<PlayingState>;
    fn set_state(&mut self, state: PlayingState) -> &mut Self;
    fn buffer(&self) -> Option<BufferState>;
    fn set_buffer(&mut self, state: BufferState) -> &mut Self;
}

impl QueueRendererStateExt for QueueRendererState {
    fn state(&self) -> Option<PlayingState> {
        self.playing_state.and_then(|i| PlayingState::try_from(i).ok())
    }
    fn set_state(&mut self, state: PlayingState) -> &mut Self {
        self.playing_state = Some(state.into());
        self
    }
    fn buffer(&self) -> Option<BufferState> {
        self.buffer_state.and_then(|i| BufferState::try_from(i).ok())
    }
    fn set_buffer(&mut self, state: BufferState) -> &mut Self {
        self.buffer_state = Some(state.into());
        self
    }
}

pub trait SetStateExt {
    fn state(&self) -> Option<PlayingState>;
}

impl SetStateExt for cmd::SetState {
    fn state(&self) -> Option<PlayingState> {
        self.playing_state.and_then(|i| PlayingState::try_from(i).ok())
    }
}

pub trait LoopModeSetExt {
    fn loop_mode(&self) -> Option<LoopMode>;
}

impl LoopModeSetExt for notify::LoopModeSet {
    fn loop_mode(&self) -> Option<LoopMode> {
        self.mode.and_then(|i| LoopMode::try_from(i).ok())
    }
}

pub trait PositionExt {
    fn now(value: u32) -> Position;
}

impl PositionExt for Position {
    fn now(value: u32) -> Position {
        Position {
            timestamp: Some(now_ms()),
            value: Some(value),
        }
    }
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system time before Unix epoch")
        .as_millis() as u64
}
