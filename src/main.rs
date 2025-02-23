mod gui;
mod package_manager;
pub mod util;

use anyhow::Result;
use clap::Parser;
use destiny_pkg::{GameVersion, PackageManager, TagHash};
use eframe::egui::ViewportBuilder;
use env_logger::Env;
use game_detector::InstalledGame;
use gui::AzilisApp;
use log::info;
use package_manager::{initialize_package_manager, package_manager};
use rrise::{
    AkResult, memory_mgr, music_engine,
    settings::{
        self, AkDeviceSettings, AkInitSettings, AkMemSettings, AkPlatformInitSettings,
        AkStreamMgrSettings,
    },
    sound_engine::{
        self, add_default_listener, clear_banks, is_initialized, register_game_obj, stop_all,
        unregister_all_game_obj,
    },
    stream_mgr,
};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

pub fn parse_taghash(s: &str) -> Result<TagHash, String> {
    const HEX_PREFIX: &str = "0x";
    const HEX_PREFIX_UPPER: &str = "0X";
    const HEX_PREFIX_LEN: usize = HEX_PREFIX.len();

    let result = if s.starts_with(HEX_PREFIX) || s.starts_with(HEX_PREFIX_UPPER) {
        u32::from_str_radix(&s[HEX_PREFIX_LEN..], 16)
    } else {
        u32::from_str_radix(s, 16)
    }
    .map(|v| TagHash(u32::from_be(v)));

    result.map_err(|e| e.to_string())
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None, disable_version_flag(true))]
struct Args {
    /// Path to packages directory
    packages_path: Option<String>,

    /// Game version for the specified packages directory
    #[arg(short, long, value_enum)]
    version: Option<GameVersion>,

    /// Manually load a bank by TagHash
    #[arg(short, long, value_parser = parse_taghash)]
    bank: Option<TagHash>,
}

fn main() -> Result<()> {
    env_logger::Builder::from_env(
        Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn"),
    )
    .init();
    let args = Args::parse();

    rayon::ThreadPoolBuilder::new()
        .thread_name(|i| format!("rayon-worker-{i}"))
        .build_global()
        .unwrap();

    let packages_path = if let Some(packages_path) = args.packages_path {
        packages_path
    } else if let Some(path) = find_d2_packages_path() {
        let mut path = std::path::PathBuf::from(path);
        path.push("packages");
        path.to_str().unwrap().to_string()
    } else {
        panic!("Could not find Destiny 2 packages directory");
    };

    info!(
        "Initializing package manager for version {:?} at '{}'",
        args.version, packages_path
    );
    let ver = args.version.unwrap_or(GameVersion::Destiny2TheFinalShape);
    let pm = PackageManager::new(packages_path, ver, None).unwrap();
    initialize_package_manager(pm);

    init_sound_engine()?;
    if !is_initialized() {
        panic!("did not init")
    }

    register_game_obj(1)?;
    add_default_listener(1)?;
    register_game_obj(100)?;

    // std::thread::spawn(move || {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder::default().with_title("Azilis - Wwise 2021.1"),
        persist_window: true,
        vsync: true,
        ..Default::default()
    };
    eframe::run_native(
        "Azilis",
        native_options,
        Box::new(|cc| Ok(Box::new(AzilisApp::new(cc)))),
    )
    .unwrap();

    term_sound_engine()?;

    Ok(())
}

fn find_d2_packages_path() -> Option<String> {
    let mut installations = game_detector::find_all_games();
    installations.retain(|i| match i {
        InstalledGame::Steam(a) => a.appid == 1085660,
        InstalledGame::EpicGames(m) => m.display_name == "Destiny 2",
        InstalledGame::MicrosoftStore(p) => p.app_name == "Destiny2PCbasegame",
        _ => false,
    });

    info!("Found {} Destiny 2 installations", installations.len());

    // Sort installations, weighting Steam > Epic > Microsoft Store
    installations.sort_by_cached_key(|i| match i {
        InstalledGame::Steam(_) => 0,
        InstalledGame::EpicGames(_) => 1,
        InstalledGame::MicrosoftStore(_) => 2,
        _ => 3,
    });

    match installations.first() {
        Some(InstalledGame::Steam(a)) => Some(a.game_path.clone()),
        Some(InstalledGame::EpicGames(m)) => Some(m.install_location.clone()),
        Some(InstalledGame::MicrosoftStore(p)) => Some(p.path.clone()),
        _ => None,
    }
}

fn init_sound_engine() -> Result<(), AkResult> {
    #[cfg(feature = "profiler")]
    profiling::scope!("init_sound_engine");

    memory_mgr::init(&mut AkMemSettings::default())?;
    assert!(memory_mgr::is_initialized());
    stream_mgr::init_default_stream_mgr(
        &AkStreamMgrSettings::default(),
        &mut AkDeviceSettings::default(),
    )
    .unwrap();

    stream_mgr::set_current_language("English(US)").unwrap();
    sound_engine::init(
        &mut AkInitSettings::default(),
        &mut AkPlatformInitSettings::default(),
    )
    .unwrap();

    music_engine::init(&mut settings::AkMusicSettings::default())?;

    Ok(())
}

fn term_sound_engine() -> Result<(), AkResult> {
    #[cfg(feature = "profiler")]
    profiling::scope!("term_sound_engine");

    sound_engine::term();
    stream_mgr::term_default_stream_mgr();
    memory_mgr::term();

    Ok(())
}
