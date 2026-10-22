use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use tracing::info;

fn default_address() -> String {
    "0.0.0.0".to_string()
}

fn default_port() -> u16 {
    2333
}

fn default_password() -> String {
    "youshallnotpass".to_string()
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub server: ServerConfig,
    #[serde(default)]
    pub lavalink: Option<LavalinkBlock>,
    #[serde(default)]
    pub sources: SourcesConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    #[serde(default = "default_address")]
    pub address: String,
    #[serde(default = "default_port")]
    pub port: u16,
    #[serde(default = "default_password")]
    pub password: String,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: default_address(),
            port: default_port(),
            password: default_password(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LavalinkBlock {
    #[serde(default)]
    pub server: Option<LavalinkServerConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct LavalinkServerConfig {
    pub password: Option<String>,
    pub sources: Option<SourcesConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourcesConfig {
    #[serde(default = "default_true")]
    pub youtube: bool,
    #[serde(default = "default_true")]
    pub http: bool,
    #[serde(default = "default_true")]
    pub local: bool,
    #[serde(default = "default_true")]
    pub hls: bool,
}

impl Default for SourcesConfig {
    fn default() -> Self {
        Self {
            youtube: true,
            http: true,
            local: true,
            hls: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            lavalink: None,
            sources: SourcesConfig::default(),
            logging: LoggingConfig::default(),
        }
    }
}

pub const DEFAULT_CONFIG_YML: &str = r#"# =======================================================
# RustLink Configuration File (Lavalink v4 Compatible)
# =======================================================

server:
  port: 2333
  address: 0.0.0.0
  password: "youshallnotpass"

# Supported audio sources
sources:
  youtube: true
  http: true
  local: true
  hls: true

# Logging level (trace, debug, info, warn, error)
logging:
  level: "info"
"#;

impl AppConfig {
    pub fn load_or_create() -> Self {
        let config_paths = ["application.yml", "application.yaml", "config.yml"];

        for path_str in config_paths {
            let path = Path::new(path_str);
            if path.exists() {
                if let Ok(content) = fs::read_to_string(path) {
                    match serde_yaml::from_str::<AppConfig>(&content) {
                        Ok(cfg) => {
                            info!("Loaded configuration from {path_str}");
                            return cfg;
                        }
                        Err(e) => {
                            eprintln!("Warning: Failed to parse {path_str}: {e}. Falling back to defaults.");
                        }
                    }
                }
            }
        }

        // If no config file exists, create a default application.yml
        let default_path = Path::new("application.yml");
        if let Err(e) = fs::write(default_path, DEFAULT_CONFIG_YML) {
            eprintln!("Notice: Could not write default application.yml: {e}");
        } else {
            info!("Created default configuration file: application.yml");
        }

        AppConfig::default()
    }

    pub fn get_address(&self) -> String {
        env::var("RUSTLINK_SERVER_HOST")
            .or_else(|_| env::var("LAVALINK_SERVER_HOST"))
            .unwrap_or_else(|_| self.server.address.clone())
    }

    pub fn get_port(&self) -> u16 {
        env::var("RUSTLINK_SERVER_PORT")
            .or_else(|_| env::var("LAVALINK_SERVER_PORT"))
            .or_else(|_| env::var("SERVER_PORT"))
            .or_else(|_| env::var("PORT"))
            .ok()
            .and_then(|p| p.parse::<u16>().ok())
            .unwrap_or(self.server.port)
    }

    pub fn get_password(&self) -> String {
        if let Ok(p) = env::var("RUSTLINK_SERVER_PASSWORD") {
            return p;
        }
        if let Ok(p) = env::var("LAVALINK_SERVER_PASSWORD") {
            return p;
        }
        if let Some(ref l) = self.lavalink {
            if let Some(ref s) = l.server {
                if let Some(ref pass) = s.password {
                    return pass.clone();
                }
            }
        }
        self.server.password.clone()
    }

    pub fn get_source_list(&self) -> Vec<String> {
        let mut list = Vec::new();
        let sources = if let Some(ref l) = self.lavalink {
            if let Some(ref s) = l.server {
                s.sources.as_ref().unwrap_or(&self.sources)
            } else {
                &self.sources
            }
        } else {
            &self.sources
        };

        if sources.youtube {
            list.push("youtube".to_string());
        }
        if sources.http {
            list.push("http".to_string());
        }
        if sources.local {
            list.push("local".to_string());
        }
        if sources.hls {
            list.push("hls".to_string());
        }

        list
    }
}
