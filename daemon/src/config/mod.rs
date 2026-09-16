use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PairedDevice {
    pub device_id: String,
    pub device_name: String,
    pub paired_at: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonConfig {
    pub server_name: String,
    pub port: u16,
    pub server_secret: String,
    pub paired_devices: Vec<PairedDevice>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            server_name: "TiVarch Ultra Remote".to_string(),
            port: 9000,
            server_secret: Uuid::new_v4().to_string(),
            paired_devices: Vec::new(),
        }
    }
}

pub struct ConfigManager {
    config_path: PathBuf,
    pub config: DaemonConfig,
}

impl ConfigManager {
    pub fn default_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("tivarch")
            .join("remote-config.json")
    }

    pub fn load_or_create() -> Result<Self> {
        let path = Self::default_path();
        Self::load_or_create_from_path(&path)
    }

    pub fn load_or_create_from_path(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
        }

        let config = if path.exists() {
            let data = fs::read_to_string(path)
                .with_context(|| format!("Failed to read config file from {:?}", path))?;
            serde_json::from_str(&data).unwrap_or_else(|_| {
                let default_cfg = DaemonConfig::default();
                let _ = fs::write(path, serde_json::to_string_pretty(&default_cfg).unwrap());
                default_cfg
            })
        } else {
            let default_cfg = DaemonConfig::default();
            let data = serde_json::to_string_pretty(&default_cfg)?;
            fs::write(path, data)
                .with_context(|| format!("Failed to write initial config to {:?}", path))?;
            info!("Created new configuration file at {:?}", path);
            default_cfg
        };

        Ok(Self {
            config_path: path.to_path_buf(),
            config,
        })
    }

    pub fn save(&self) -> Result<()> {
        let data = serde_json::to_string_pretty(&self.config)?;
        fs::write(&self.config_path, data)
            .with_context(|| format!("Failed to persist config to {:?}", self.config_path))?;
        Ok(())
    }

    pub fn is_device_paired(&self, device_id: &str) -> bool {
        self.config
            .paired_devices
            .iter()
            .any(|d| d.device_id == device_id)
    }

    pub fn register_device(&mut self, device_id: String, device_name: String) -> Result<()> {
        if !self.is_device_paired(&device_id) {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();

            self.config.paired_devices.push(PairedDevice {
                device_id,
                device_name,
                paired_at: now,
            });
            self.save()?;
            info!("Device registered and saved to remote-config.json");
        }
        Ok(())
    }

    #[allow(dead_code)]
    pub fn unpair_device(&mut self, device_id: &str) -> Result<bool> {
        let initial_len = self.config.paired_devices.len();
        self.config
            .paired_devices
            .retain(|d| d.device_id != device_id);
        if self.config.paired_devices.len() != initial_len {
            self.save()?;
            Ok(true)
        } else {
            Ok(false)
        }
    }
}