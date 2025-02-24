use directories::ProjectDirs;
use eframe::egui::{ahash::HashSet, mutex::RwLock};
use lazy_static::lazy_static;
use log::{error, info};
use serde::{Deserialize, Serialize};

lazy_static! {
    pub static ref CONFIGURATION: RwLock<Config> = RwLock::new(Config::default());
}

pub fn try_persist() -> anyhow::Result<()> {
    let pd = ProjectDirs::from("net", "nblock", "Azilis")
        .expect("Failed to get application directories");
    std::fs::create_dir_all(pd.config_dir()).expect("Failed to create config directory");
    std::fs::create_dir_all(pd.config_local_dir())
        .expect("Failed to create local config directory");
    Ok(std::fs::write(
        pd.config_dir().join("config.yml"),
        serde_yaml::to_string(&*CONFIGURATION.read())?,
    )?)
}

pub fn persist() {
    if let Err(e) = try_persist() {
        error!("Failed to write config: {e}");
    } else {
        info!("Config written successfully!");
    }
}

pub fn load() {
    let pd = ProjectDirs::from("net", "nblock", "Azilis")
        .expect("Failed to get application directories");
    std::fs::create_dir_all(pd.config_dir()).expect("Failed to create config directory");
    std::fs::create_dir_all(pd.config_local_dir())
        .expect("Failed to create local config directory");
    if let Ok(c) = std::fs::read_to_string(pd.config_dir().join("config.yml")) {
        match serde_yaml::from_str(&c) {
            Ok(config) => {
                with_mut(|c| *c = config);
            }
            Err(e) => {
                error!("Failed to parse config: {e}");
            }
        }
    } else {
        info!("No config found, creating a new one");
        persist();
    }
}

pub fn with<F, T>(f: F) -> T
where
    F: FnOnce(&Config) -> T,
{
    f(&CONFIGURATION.read())
}

pub fn with_mut<F, T>(f: F) -> T
where
    F: FnOnce(&mut Config) -> T,
{
    f(&mut CONFIGURATION.write())
}

#[macro_export]
macro_rules! config {
    () => {
        ($crate::config::CONFIGURATION.read())
    };
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub window: WindowConfig,
    pub audio: AudioConfig,
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct AudioConfig {
    pub volume: f32,
}

impl Default for AudioConfig {
    fn default() -> Self {
        AudioConfig { volume: 0.5 }
    }
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct WindowConfig {
    pub width: f32,
    pub height: f32,
    pub pos_x: f32,
    pub pos_y: f32,
    pub maximised: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        WindowConfig {
            width: 1600.0,
            height: 900.0,
            pos_x: 0.0,
            pos_y: 0.0,
            maximised: false,
        }
    }
}
