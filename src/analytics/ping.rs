#[allow(dead_code)]
#[allow(dead_code)]
use anyhow::Result;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

#[allow(dead_code)]
pub struct Analytics {
    device_id: String,
    config_dir: PathBuf,
}

#[allow(dead_code)]
impl Analytics {
    pub fn new() -> Result<Self> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("orion");
        fs::create_dir_all(&config_dir)?;

        let device_id_path = config_dir.join(".device_id");
        let device_id = if device_id_path.exists() {
            fs::read_to_string(&device_id_path)?.trim().to_string()
        } else {
            let id = Uuid::new_v4().to_string();
            fs::write(&device_id_path, &id)?;
            id
        };

        Ok(Self { device_id, config_dir })
    }

    pub fn ping(&self, event: &str) -> Result<()> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)?
            .as_secs();

        let log_path = self.config_dir.join("pings.jsonl");
        let entry = serde_json::json!({
            "device_id": self.device_id,
            "event": event,
            "timestamp": timestamp,
            "version": env!("CARGO_PKG_VERSION"),
        });

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&log_path)?;

        writeln!(file, "{}", entry)?;
        Ok(())
    }

    pub fn increment_download_count(&self) -> Result<u64> {
        let count_path = self.config_dir.join(".downloads");
        let count: u64 = if count_path.exists() {
            fs::read_to_string(&count_path)?.trim().parse().unwrap_or(0)
        } else {
            0
        };
        let new_count = count + 1;
        fs::write(&count_path, new_count.to_string())?;
        Ok(new_count)
    }
}

impl Default for Analytics {
    fn default() -> Self {
        Self::new().expect("Failed to initialize analytics")
    }
}
