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
    #[serde(default = "default_autostart")]
    pub autostart: bool,
}

fn default_autostart() -> bool {
    true
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
                expanded_width: 775,
                expanded_height: 230,
                hover_debounce_ms: 300,
                autostart: true,
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

    pub fn save(&self) {
        if let Some(path) = Self::get_config_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(serialized) = toml::to_string_pretty(self) {
                let _ = fs::write(&path, serialized);
            }
        }
    }

    pub fn update_pomodoro_durations(work_min: u32, break_min: u32) {
        let mut config = Self::load();
        config.pomodoro.work_duration_min = work_min;
        config.pomodoro.break_duration_min = break_min;
        config.save();
    }

    pub fn get_config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut p| {
            p.push("open-notch");
            p.push("config.toml");
            p
        })
    }
}

pub fn get_autostart_desktop_path() -> Option<PathBuf> {
    dirs::config_dir().map(|mut p| {
        p.push("autostart");
        p.push("open-notch.desktop");
        p
    })
}

pub fn is_autostart_enabled() -> bool {
    if let Some(path) = get_autostart_desktop_path() {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if content.contains("X-GNOME-Autostart-enabled=false") {
                    return false;
                }
            }
            return true;
        }
    }
    let cfg = Config::load();
    cfg.general.autostart
}

pub fn set_autostart_enabled(enabled: bool) {
    let mut cfg = Config::load();
    cfg.general.autostart = enabled;
    cfg.save();

    if let Some(path) = get_autostart_desktop_path() {
        if enabled {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let exec_path = std::env::current_exe()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|_| "open-notch".to_string());

            let content = format!(
                "[Desktop Entry]\n\
                 Name=OpenNotch\n\
                 GenericName=Desktop Dynamic Notch\n\
                 Comment=A sleek, fluid, and dynamic notch overlay for Linux desktops\n\
                 Exec={}\n\
                 Icon=open-notch\n\
                 Terminal=false\n\
                 Type=Application\n\
                 Categories=Utility;GTK;\n\
                 Keywords=notch;dynamic;island;shelf;media;clipboard;pomodoro;dock;overlay;\n\
                 StartupNotify=false\n\
                 StartupWMClass=open-notch\n\
                 X-GNOME-Autostart-enabled=true\n\
                 X-GNOME-Autostart-Delay=2\n\
                 X-KDE-autostart-after=panel\n",
                exec_path
            );
            let _ = fs::write(&path, content);
        } else {
            if path.exists() {
                let _ = fs::remove_file(&path);
            }
        }
    }
}

pub fn ensure_default_autostart() {
    let cfg = Config::load();
    if cfg.general.autostart {
        if let Some(path) = get_autostart_desktop_path() {
            if !path.exists() {
                set_autostart_enabled(true);
            }
        }
    }
}
