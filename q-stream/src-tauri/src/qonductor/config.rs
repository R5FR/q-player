use md5::{Digest, Md5};
use uuid::Uuid;

use super::proto::qconnect::DeviceType;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(i32)]
pub enum AudioQuality {
    Mp3 = 1,
    FlacLossless = 2,
    HiRes96 = 3,
    HiRes192 = 4,
}

#[derive(Debug, Clone)]
pub struct DeviceConfig {
    pub device_uuid: [u8; 16],
    pub friendly_name: String,
    pub device_type: DeviceType,
    pub brand: String,
    pub model: String,
    pub max_audio_quality: AudioQuality,
    pub app_id: String,
}

impl DeviceConfig {
    pub fn new(friendly_name: impl Into<String>, app_id: impl Into<String>) -> Self {
        let name = friendly_name.into();
        let mut hasher = Md5::new();
        hasher.update(format!("qonductor:{}", name).as_bytes());
        let uuid: [u8; 16] = hasher.finalize().into();

        Self {
            device_uuid: uuid,
            friendly_name: name,
            device_type: DeviceType::Speaker,
            brand: "Qonductor".to_string(),
            model: "Qonductor Rust".to_string(),
            max_audio_quality: AudioQuality::HiRes192,
            app_id: app_id.into(),
        }
    }

    pub fn with_uuid(
        device_uuid: [u8; 16],
        friendly_name: impl Into<String>,
        app_id: impl Into<String>,
    ) -> Self {
        Self {
            device_uuid,
            friendly_name: friendly_name.into(),
            device_type: DeviceType::Speaker,
            brand: "Qonductor".to_string(),
            model: "Qonductor Rust".to_string(),
            max_audio_quality: AudioQuality::HiRes192,
            app_id: app_id.into(),
        }
    }

    pub fn uuid_hex(&self) -> String {
        Uuid::from_bytes(self.device_uuid).simple().to_string()
    }

    pub fn uuid_formatted(&self) -> String {
        Uuid::from_bytes(self.device_uuid).hyphenated().to_string()
    }
}

#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub session_id: String,
    pub ws_endpoint: String,
    pub ws_jwt: String,
    #[allow(dead_code)]
    pub ws_jwt_exp: u64,
    pub api_jwt: String,
    #[allow(dead_code)]
    pub api_jwt_exp: u64,
}
