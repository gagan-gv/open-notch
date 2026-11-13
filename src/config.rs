use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub general: GeneralConfig,
    pub modules: ModulesConfig,
    pub pomodoro: PomodoroConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralConfig {
    pub collapsed_width: i32,
    pub collapsed_height: i32,
    pub expanded_width: i32,
    pub expanded_height: i32,
    pub hover_debounce_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModulesConfig {
    pub default_view: String, // "calendar", "pomodoro", "notes"
    pub enable_media: bool,
    pub enable_calendar: bool,
    pub enable_pomodoro: bool,
    pub enable_notes: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PomodoroConfig {
    pub work_duration_min: u32,
    pub break_duration_min: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                collapsed_width: 180,
                collapsed_height: 32,
                expanded_width: 540,
                expanded_height: 380,
                hover_debounce_ms: 400,
            },
            modules: ModulesConfig {
                default_view: "calendar".to_string(),
                enable_media: true,
                enable_calendar: true,
                enable_pomodoro: true,
                enable_notes: true,
            },
            pomodoro: PomodoroConfig {
                work_duration_min: 25,
                break_duration_min: 5,
            },
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();

        if let Some(path) = config_path {
            if path.exists() {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(config) = toml::from_str::<Config>(&content) {
                        return config;
                    }
                }
            } else {
                // Ensure parent directory exists and write default config
                if let Some(parent) = path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let default_config = Config::default();
                if let Ok(serialized) = toml::to_string_pretty(&default_config) {
                    let _ = fs::write(&path, serialized);
                }
                return default_config;
            }
        }

        Config::default()
    }

    pub fn get_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("open-notch");
            p.push("config.toml");
            p
        })
    }
}
