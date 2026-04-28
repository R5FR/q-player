use tokio::sync::mpsc;
use tracing::{error, info, warn};

use super::config::{DeviceConfig, SessionInfo};
use super::discovery::{DeviceRegistry, DeviceSelected};
use super::qconnect::spawn_session;
use super::session::DeviceSession;
use super::{Error, Result};

pub struct SessionManager {
    registry: DeviceRegistry,
    device_rx: mpsc::Receiver<DeviceSelected>,
}

impl SessionManager {
    pub async fn start(port: u16) -> Result<Self> {
        let (registry, device_rx) = DeviceRegistry::start(port).await?;

        Ok(Self {
            registry,
            device_rx,
        })
    }

    pub async fn add_device(&self, config: DeviceConfig) -> Result<DeviceSession> {
        self.registry.add_device(config).await
    }

    pub async fn remove_device(&mut self, device_uuid: &[u8; 16]) -> Result<()> {
        self.registry.remove_device(device_uuid).await
    }

    pub async fn devices(&self) -> Vec<DeviceConfig> {
        self.registry.devices().await
    }

    pub async fn shutdown(&self) {
        self.registry.shutdown().await;
    }

    pub async fn run(&mut self) -> Result<()> {
        info!("SessionManager starting");

        loop {
            match self.device_rx.recv().await {
                Some(s) => {
                    if let Err(e) = self.handle_device_selected(s).await {
                        error!(error = %e, "Failed to handle device selection");
                    }
                }
                None => {
                    warn!("Device selection channel closed");
                    break;
                }
            }
        }

        Ok(())
    }

    pub async fn connect_proactive(&self, session_info: SessionInfo) -> Result<()> {
        let devices = self.registry.devices().await;
        let device_config = devices
            .into_iter()
            .next()
            .ok_or_else(|| Error::Discovery("No devices registered".to_string()))?;

        let event_tx = self
            .registry
            .get_event_tx(&device_config.device_uuid)
            .await
            .ok_or_else(|| Error::Discovery("Device event channel not found".to_string()))?;

        let shared_command_tx = self
            .registry
            .get_command_tx(&device_config.device_uuid)
            .await
            .ok_or_else(|| Error::Discovery("Device command channel not found".to_string()))?;

        let (command_tx, command_rx) = tokio::sync::mpsc::channel(100);
        *shared_command_tx.write().await = Some(command_tx);

        info!(
            device = %device_config.friendly_name,
            "Proactively connecting to Qobuz Connect WebSocket"
        );

        spawn_session(&session_info, &device_config, event_tx, command_rx).await
    }

    async fn handle_device_selected(&mut self, selected: DeviceSelected) -> Result<()> {
        let device_config = self
            .registry
            .get_device(&selected.device_uuid)
            .await
            .ok_or_else(|| Error::Discovery("Device not found".to_string()))?;

        let event_tx = self
            .registry
            .get_event_tx(&selected.device_uuid)
            .await
            .ok_or_else(|| Error::Discovery("Device event channel not found".to_string()))?;

        let shared_command_tx = self
            .registry
            .get_command_tx(&selected.device_uuid)
            .await
            .ok_or_else(|| Error::Discovery("Device command channel not found".to_string()))?;

        let (command_tx, command_rx) = tokio::sync::mpsc::channel(100);
        *shared_command_tx.write().await = Some(command_tx);

        info!(
            device = %device_config.friendly_name,
            "Device selected, creating session"
        );

        spawn_session(&selected.session_info, &device_config, event_tx, command_rx).await
    }
}
