use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const STATE_FILE: &str = "chnroutes-state.json";

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    pub source: String,
    pub interface_index: u32,
    pub routes: Vec<String>,
    pub updated_at: u64,
}

impl State {
    pub fn new(source: String, interface_index: u32, routes: Vec<String>) -> Self {
        let updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            source,
            interface_index,
            routes,
            updated_at,
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let path = Self::path();

        // 自动创建上级目录（防路径不存在报错）
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let data = serde_json::to_string_pretty(self)?;
        fs::write(path, data)?;
        Ok(())
    }

    pub fn load() -> Option<Self> {
        let path = Self::path();
        if !path.exists() {
            return None;
        }

        let data = fs::read_to_string(path).ok()?;
        serde_json::from_str(&data).ok()
    }

    pub fn remove() -> std::io::Result<()> {
        let path = Self::path();
        if path.exists() {
            fs::remove_file(path)?;
        }
        Ok(())
    }

    fn path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(STATE_FILE)
    }
}
