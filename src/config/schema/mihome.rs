use serde::{Deserialize, Serialize};
use typeshare::typeshare;

#[typeshare]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct MiHomeConfig {
    pub enabled: bool,
    pub api_url: String,
    pub api_key: String,
}

impl Default for MiHomeConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_url: "http://localhost:7123".to_string(),
            api_key: String::new(),
        }
    }
}