pub mod config;
pub mod credentials;
pub mod error;
pub mod event;
pub mod manager;
pub mod msg;
pub mod session;
pub mod types;

pub(crate) mod connection;
pub(crate) mod discovery;
pub mod qconnect;

pub use manager::SessionManager;
pub use event::{ActivationState, Command, Notification, Responder, SessionEvent};
pub use session::{DeviceSession, SessionCommand};
pub use proto::qconnect::{BufferState, DeviceType, LoopMode, PlayingState};
pub use config::{AudioQuality, DeviceConfig, SessionInfo};
pub use discovery::DeviceTypeExt;
pub use connection::format_qconnect_message;

pub use error::Error;
pub use types::*;

pub type Result<T> = std::result::Result<T, Error>;

pub mod proto {
    #[allow(clippy::all)]
    pub mod qconnect {
        include!("proto/qconnect.rs");
    }
    #[allow(clippy::all)]
    pub mod ws {
        include!("proto/ws.rs");
    }
}
