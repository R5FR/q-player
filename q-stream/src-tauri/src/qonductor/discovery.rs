use std::collections::HashMap;
use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use mdns_sd::{ServiceDaemon, ServiceInfo};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::{RwLock, mpsc};
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use super::config::{AudioQuality, DeviceConfig, SessionInfo};
use super::event::SessionEvent;
use super::proto::qconnect::DeviceType;
use super::session::{DeviceSession, SharedCommandTx};
use super::{Error, Result};

const QCONNECT_SDK_VERSION: &str = "0.9.6";

pub trait DeviceTypeExt {
    fn as_str(&self) -> &'static str;
}

impl DeviceTypeExt for DeviceType {
    fn as_str(&self) -> &'static str {
        match self {
            DeviceType::Unknown => "UNKNOWN",
            DeviceType::Speaker => "SPEAKER",
            DeviceType::Speakerbox => "SPEAKERBOX",
            DeviceType::Tv => "TV",
            DeviceType::Speakerbox2 => "SPEAKERBOX2",
            DeviceType::Laptop => "LAPTOP",
            DeviceType::Phone => "PHONE",
            DeviceType::GoogleCast => "GOOGLE_CAST",
            DeviceType::Headphones => "HEADPHONES",
            DeviceType::Tablet => "TABLET",
        }
    }
}

fn audio_quality_display(quality: AudioQuality) -> &'static str {
    match quality {
        AudioQuality::Mp3 => "MP3",
        AudioQuality::FlacLossless => "LOSSLESS",
        AudioQuality::HiRes96 => "HIRES_L2",
        AudioQuality::HiRes192 => "HIRES_L3",
    }
}

#[derive(Debug, Clone)]
pub struct DeviceSelected {
    pub device_uuid: [u8; 16],
    pub session_info: SessionInfo,
}

#[derive(Debug, Serialize)]
struct DisplayInfoResponse {
    #[serde(rename = "type")]
    device_type: String,
    friendly_name: String,
    model_display_name: String,
    brand_display_name: String,
    serial_number: String,
    max_audio_quality: String,
}

#[derive(Debug, Serialize)]
struct ConnectInfoResponse {
    current_session_id: String,
    app_id: String,
}

#[derive(Debug, Deserialize)]
struct JwtInfo {
    endpoint: Option<String>,
    jwt: String,
    exp: u64,
}

#[derive(Debug, Deserialize)]
struct ConnectRequest {
    session_id: String,
    jwt_qconnect: JwtInfo,
    jwt_api: JwtInfo,
}

#[derive(Debug, Serialize)]
struct ConnectResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

struct RegisteredDevice {
    config: DeviceConfig,
    service_info: ServiceInfo,
    current_session_id: Option<String>,
    event_tx: mpsc::Sender<SessionEvent>,
    command_tx: SharedCommandTx,
}

struct AppState {
    devices: RwLock<HashMap<[u8; 16], RegisteredDevice>>,
    event_tx: mpsc::Sender<DeviceSelected>,
}

async fn get_display_info(
    State(state): State<Arc<AppState>>,
    Path(uuid_hex): Path<String>,
) -> std::result::Result<Json<DisplayInfoResponse>, StatusCode> {
    let uuid = Uuid::parse_str(&uuid_hex)
        .ok()
        .map(|u| *u.as_bytes())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let devices = state.devices.read().await;
    let device = devices.get(&uuid).ok_or(StatusCode::NOT_FOUND)?;

    debug!(device = %device.config.friendly_name, "GET get-display-info");

    Ok(Json(DisplayInfoResponse {
        device_type: device.config.device_type.as_str().to_string(),
        friendly_name: device.config.friendly_name.clone(),
        model_display_name: device.config.model.clone(),
        brand_display_name: device.config.brand.clone(),
        serial_number: device.config.uuid_formatted(),
        max_audio_quality: audio_quality_display(device.config.max_audio_quality).to_string(),
    }))
}

async fn get_connect_info(
    State(state): State<Arc<AppState>>,
    Path(uuid_hex): Path<String>,
) -> std::result::Result<Json<ConnectInfoResponse>, StatusCode> {
    let uuid = Uuid::parse_str(&uuid_hex)
        .ok()
        .map(|u| *u.as_bytes())
        .ok_or(StatusCode::BAD_REQUEST)?;

    let devices = state.devices.read().await;
    let device = devices.get(&uuid).ok_or(StatusCode::NOT_FOUND)?;

    debug!(device = %device.config.friendly_name, "GET get-connect-info");

    Ok(Json(ConnectInfoResponse {
        current_session_id: device.current_session_id.clone().unwrap_or_default(),
        app_id: device.config.app_id.clone(),
    }))
}

async fn connect_to_qconnect(
    State(state): State<Arc<AppState>>,
    Path(uuid_hex): Path<String>,
    Json(req): Json<ConnectRequest>,
) -> std::result::Result<Json<ConnectResponse>, StatusCode> {
    let uuid = Uuid::parse_str(&uuid_hex)
        .ok()
        .map(|u| *u.as_bytes())
        .ok_or(StatusCode::BAD_REQUEST)?;

    {
        let mut devices = state.devices.write().await;
        let device = devices.get_mut(&uuid).ok_or(StatusCode::NOT_FOUND)?;

        info!(
            device = %device.config.friendly_name,
            session_id = %req.session_id,
            "POST connect-to-qconnect"
        );

        device.current_session_id = Some(req.session_id.clone());
    }

    let ws_endpoint = req
        .jwt_qconnect
        .endpoint
        .unwrap_or_else(|| "wss://play.qobuz.com/ws".to_string());

    let session_info = SessionInfo {
        session_id: req.session_id,
        ws_endpoint,
        ws_jwt: req.jwt_qconnect.jwt,
        ws_jwt_exp: req.jwt_qconnect.exp,
        api_jwt: req.jwt_api.jwt,
        api_jwt_exp: req.jwt_api.exp,
    };

    if let Err(e) = state
        .event_tx
        .send(DeviceSelected {
            device_uuid: uuid,
            session_info,
        })
        .await
    {
        error!("Failed to send device selected event: {}", e);
        return Ok(Json(ConnectResponse {
            success: false,
            error: Some("Internal error".to_string()),
        }));
    }

    Ok(Json(ConnectResponse {
        success: true,
        error: None,
    }))
}

const REANNOUNCE_INTERVAL: Duration = Duration::from_secs(5);

pub struct DeviceRegistry {
    state: Arc<AppState>,
    http_port: u16,
    mdns_daemon: ServiceDaemon,
}

impl DeviceRegistry {
    pub async fn start(port: u16) -> Result<(Self, mpsc::Receiver<DeviceSelected>)> {
        let (event_tx, event_rx) = mpsc::channel(16);

        let state = Arc::new(AppState {
            devices: RwLock::new(HashMap::new()),
            event_tx,
        });

        let addr = SocketAddr::from(([0, 0, 0, 0], port));
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| Error::Discovery(format!("Failed to bind: {e}")))?;

        let local_addr = listener
            .local_addr()
            .map_err(|e| Error::Discovery(format!("Failed to get local addr: {e}")))?;

        info!("Device registry HTTP server listening on {}", local_addr);

        let app = Router::new()
            .route("/devices/{uuid}/get-display-info", get(get_display_info))
            .route("/devices/{uuid}/get-connect-info", get(get_connect_info))
            .route(
                "/devices/{uuid}/connect-to-qconnect",
                post(connect_to_qconnect),
            )
            .with_state(state.clone());

        tokio::spawn(async move {
            if let Err(e) = axum::serve(listener, app).await {
                error!("HTTP server error: {}", e);
            }
        });

        let mdns_daemon = ServiceDaemon::new()
            .map_err(|e| Error::Mdns(format!("Failed to create mDNS daemon: {e}")))?;

        info!("mDNS daemon started");

        let state_clone = state.clone();
        let daemon_clone = mdns_daemon.clone();
        tokio::spawn(async move {
            Self::reannounce_loop(state_clone, daemon_clone).await;
        });

        Ok((
            Self {
                state,
                http_port: local_addr.port(),
                mdns_daemon,
            },
            event_rx,
        ))
    }

    async fn reannounce_loop(state: Arc<AppState>, daemon: ServiceDaemon) {
        loop {
            tokio::time::sleep(REANNOUNCE_INTERVAL).await;

            let devices = state.devices.read().await;
            for device in devices.values() {
                if let Err(e) = daemon.register(device.service_info.clone()) {
                    debug!(
                        error = %e,
                        device = %device.config.friendly_name,
                        "Failed to re-announce service"
                    );
                }
            }
        }
    }

    pub async fn add_device(&self, config: DeviceConfig) -> Result<DeviceSession> {
        let uuid = config.device_uuid;
        let uuid_str = config.uuid_formatted();

        info!(
            device = %config.friendly_name,
            uuid = %uuid_str,
            "Registering device"
        );

        let (event_tx, event_rx) = mpsc::channel(100);
        let command_tx: SharedCommandTx = Arc::new(RwLock::new(None));
        let service_info = self.register_mdns(&config, &uuid_str)?;

        let registered = RegisteredDevice {
            config,
            service_info,
            current_session_id: None,
            event_tx,
            command_tx: command_tx.clone(),
        };

        self.state.devices.write().await.insert(uuid, registered);

        Ok(DeviceSession::new(event_rx, command_tx))
    }

    pub async fn remove_device(&self, device_uuid: &[u8; 16]) -> Result<()> {
        let mut devices = self.state.devices.write().await;

        if let Some(device) = devices.remove(device_uuid) {
            info!(device = %device.config.friendly_name, "Unregistering device");
            if let Err(e) = self.mdns_daemon.unregister(device.service_info.get_fullname()) {
                warn!(error = %e, "Failed to unregister mDNS service");
            }
        } else {
            warn!(uuid = %Uuid::from_bytes(*device_uuid), "Device not found for removal");
        }

        Ok(())
    }

    pub async fn get_device(&self, device_uuid: &[u8; 16]) -> Option<DeviceConfig> {
        self.state
            .devices
            .read()
            .await
            .get(device_uuid)
            .map(|d| d.config.clone())
    }

    pub async fn get_event_tx(&self, device_uuid: &[u8; 16]) -> Option<mpsc::Sender<SessionEvent>> {
        self.state
            .devices
            .read()
            .await
            .get(device_uuid)
            .map(|d| d.event_tx.clone())
    }

    pub async fn get_command_tx(&self, device_uuid: &[u8; 16]) -> Option<SharedCommandTx> {
        self.state
            .devices
            .read()
            .await
            .get(device_uuid)
            .map(|d| d.command_tx.clone())
    }

    pub async fn devices(&self) -> Vec<DeviceConfig> {
        self.state
            .devices
            .read()
            .await
            .values()
            .map(|d| d.config.clone())
            .collect()
    }

    fn register_mdns(&self, config: &DeviceConfig, uuid_hex: &str) -> Result<ServiceInfo> {
        let service_type = "_qobuz-connect._tcp.local.";
        let instance_name = &config.friendly_name;

        let ipv4 = get_local_ipv4().unwrap_or(Ipv4Addr::LOCALHOST);
        let ip_str = ipv4.to_string();
        let hostname = "qonductor.local.";

        let properties = [
            ("path", format!("/devices/{}", uuid_hex)),
            ("type", config.device_type.as_str().to_string()),
            ("Name", config.friendly_name.clone()),
            ("device_uuid", uuid_hex.to_string()),
            ("sdk_version", QCONNECT_SDK_VERSION.to_string()),
        ];

        let service_info = ServiceInfo::new(
            service_type,
            instance_name,
            hostname,
            &ip_str,
            self.http_port,
            &properties[..],
        )
        .map_err(|e| Error::Mdns(format!("Failed to create service info: {e}")))?;

        info!(device = %config.friendly_name, port = self.http_port, "Registering mDNS service");

        self.mdns_daemon
            .register(service_info.clone())
            .map_err(|e| Error::Mdns(format!("Failed to register mDNS service: {e}")))?;

        info!(device = %config.friendly_name, "mDNS service registered");

        Ok(service_info)
    }

    pub async fn shutdown(&self) {
        info!("Shutting down device registry");

        let devices = self.state.devices.read().await;
        for device in devices.values() {
            info!(device = %device.config.friendly_name, "Unregistering mDNS service");
            if let Ok(receiver) = self.mdns_daemon.unregister(device.service_info.get_fullname()) {
                let _ = receiver.recv_timeout(Duration::from_millis(200));
            }
        }
        drop(devices);

        if let Ok(receiver) = self.mdns_daemon.shutdown() {
            let _ = receiver.recv_timeout(Duration::from_secs(1));
        }
        info!("Device registry shutdown complete");
    }
}

fn get_local_ipv4() -> Option<Ipv4Addr> {
    use std::net::UdpSocket;
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    let addr = socket.local_addr().ok()?;
    match addr.ip() {
        std::net::IpAddr::V4(ip) => Some(ip),
        _ => None,
    }
}
