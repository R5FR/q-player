use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("authentication failed: {0}")]
    Auth(String),
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("WebSocket error: {0}")]
    WebSocket(#[from] Box<tokio_tungstenite::tungstenite::Error>),
    #[error("protobuf error: {0}")]
    Proto(#[from] prost::DecodeError),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid response: {0}")]
    InvalidResponse(String),
    #[error("track not found: {0}")]
    TrackNotFound(u64),
    #[error("stream error: {0}")]
    Stream(String),
    #[error("session expired")]
    SessionExpired,
    #[error("connection closed")]
    ConnectionClosed,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("mDNS error: {0}")]
    Mdns(String),
    #[error("discovery error: {0}")]
    Discovery(String),
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("session error: {0}")]
    Session(String),
}
