#![feature(let_chains)]
mod config;
mod gui;
mod package_manager;
mod util;

use anyhow::Result;
use chroma_dbg::ChromaDebug;
use clap::Parser;
use destiny_pkg::{GameVersion, PackageManager, TagHash};
use eframe::egui::{Pos2, Vec2, ViewportBuilder};
use env_logger::Env;
use game_detector::InstalledGame;
use gui::AzilisApp;
use log::info;
use package_manager::{initialize_package_manager, package_manager};
use rrise::{
    AkCallbackType, AkResult, memory_mgr, music_engine,
    settings::{
        self, AkDeviceSettings, AkInitSettings, AkMemSettings, AkPlatformInitSettings,
        AkStreamMgrSettings,
    },
    sound_engine::{
        self, add_default_listener, clear_banks, is_initialized, register_game_obj, render_audio,
        stop_all, unregister_all_game_obj,
    },
    stream_mgr,
};
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
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

#[cfg(not(feature = "profiler"))]
#[cfg(not(feature = "dhat-heap"))]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

// #[cfg(feature = "dhat-heap")]
// #[global_allocator]
// static GLOBAL: dhat::Alloc = dhat::Alloc;

const AUDIO_DEVICE_SYSTEM: u32 = 3859886410;

fn main() -> Result<()> {
    // #[cfg(feature = "dhat-heap")]
    // let _profiler = dhat::Profiler::new_heap();

    env_logger::Builder::from_env(
        Env::default().default_filter_or("info,wgpu_core=warn,wgpu_hal=warn"),
    )
    .init();

    // let should_stop = Arc::new(AtomicBool::new(false));
    // let sstop = should_stop.clone();
    // ctrlc::set_handler(move || {
    //     sstop.store(true, Ordering::SeqCst);
    // })
    // .expect("Error setting Ctrl-C handler");

    config::load();

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
    let pm = Arc::new(PackageManager::new(&packages_path, ver, None)?);
    initialize_package_manager(&pm);
    rrise::package_manager::initialize_package_manager(&pm);

    init_sound_engine()?;
    if !is_initialized() {
        panic!("did not init")
    }

    register_game_obj(1)?;
    add_default_listener(1)?;
    register_game_obj(100)?;

    let mut bank_data = Vec::new();
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("load init bnk");

        let init_tags = package_manager().get_all_by_type(26, Some(5));

        {
            let init_data = package_manager().read_tag(init_tags.first().unwrap().0)?;
            let data_len = init_data.len() as u32;
            bank_data.push(init_data);
            let id = sound_engine::load_bank_memory_view(
                bank_data[0].as_mut_ptr() as *mut std::ffi::c_void,
                data_len,
            )?;
        }
    }

    // --- OFFLINE RENDERING ---

    // let bnk_data = crate::gui::player::load_bank(&mut package_manager().read_tag(0x80BF843F)?)?;

    // // sound_engine::set_offline_rendering(true)?;
    // // sound_engine::set_offline_rendering_time(0.0)?;
    // // sound_engine::render_audio(false)?;

    // // let mut cc = sound_engine::AkChannelConfig::default();
    // // cc.set_standard(rrise::AK_SPEAKER_SETUP_2_0);
    // // let mut out_settings = rrise::AkOutputSettings {
    // //     audioDeviceShareset: AUDIO_DEVICE_SYSTEM,
    // //     idDevice: 0,
    // //     ePanningRule: rrise::AkPanningRule::AkPanningRule_Headphones,
    // //     channelConfig: cc.as_ak(),
    // // };

    // // info!("{}", cc.dbg_chroma());

    // // let _new_device_id = sound_engine::replace_output(&mut out_settings, 0)?;
    // // sound_engine::set_offline_rendering(true)?;
    // // sound_engine::render_audio(true)?;
    // // let samplerate = sound_engine::get_sample_rate();

    // // info!("sample rate: {:#?}", samplerate);
    // // let spec = hound::WavSpec {
    // //     channels: cc.num_channels as u16,
    // //     sample_rate: samplerate,
    // //     bits_per_sample: 32,
    // //     sample_format: hound::SampleFormat::Float,
    // // };
    // // TODO: flac
    // // let writer = Arc::new(eframe::egui::mutex::Mutex::new(
    // //     hound::WavWriter::create("test.wav", spec).unwrap(),
    // // ));
    // // sound_engine::register_capture_callback(
    // //     move |x| {
    // //         let sample_count = x.uValidFrames as u32 * x.channelConfig.uNumChannels();
    // //         let samples =
    // //             unsafe { std::slice::from_raw_parts(x.pData as *const f32, sample_count as usize) };
    // //         // info!("{:#?}", samples.first());
    // //         for s in samples {
    // //             writer.lock().write_sample(*s).unwrap();
    // //         }
    // //     },
    // //     _new_device_id,
    // // )?;

    // // sound_engine::set_offline_rendering_time(10.0)?;
    // rrise::game_syncs::set_switch(crate::gui::player::MUSIC_GROUP_ID, 1106066068, 100)?;
    // // TODO: Add callbacks to offline rendering so you can add a stop point
    // rrise::monitor::set_local_output(3, Some(rrise::monitoring_callback))?;
    // info!(
    //     "{}",
    //     sound_engine::PostEvent::new(100, bnk_data.play_event_id).post()?
    // );

    // loop {
    //     // if should_stop.load(Ordering::Relaxed) {
    //     //     stop_all(None);
    //     //     unregister_all_game_obj()?;
    //     //     break;
    //     // }
    //     render_audio(true)?;
    //     std::thread::sleep(std::time::Duration::from_millis(1000 / 60));
    // }

    // return Ok(());

    // --- OFFLINE RENDERING ---

    // std::thread::spawn(move || {
    let native_options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: ViewportBuilder::default()
            .with_title("Azilis - Wwise 2021.1")
            .with_inner_size(config::with(|c| Vec2::new(c.window.width, c.window.height)))
            .with_position(config::with(|c| Pos2::new(c.window.pos_x, c.window.pos_y)))
            .with_maximized(config!().window.maximised),
        persist_window: true,
        vsync: true,
        ..Default::default()
    };

    config::with_mut(|c| {
        let corrected_size = native_options.viewport.inner_size.unwrap();
        c.window.width = corrected_size.x;
        c.window.height = corrected_size.y;
    });
    config::persist();

    Ok(eframe::run_native(
        "Azilis",
        native_options,
        Box::new(|cc| Ok(Box::new(AzilisApp::new(cc)))),
    )?)

    // config::persist();

    // clear_banks().unwrap();
    // unregister_all_game_obj().unwrap();
    // term_sound_engine().unwrap();
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

    rrise::monitor::set_local_output(3, Some(rrise::monitoring_callback))?;

    memory_mgr::init(&mut AkMemSettings::default())?;
    assert!(memory_mgr::is_initialized());
    stream_mgr::init_tiger_stream_mgr(
        &AkStreamMgrSettings::default(),
        &mut AkDeviceSettings::default(),
    )?;

    // let mut cc = sound_engine::AkChannelConfig::default();
    // cc.set_standard(rrise::AK_SPEAKER_SETUP_2_0);
    // let settings_main_output = rrise::AkOutputSettings {
    //     audioDeviceShareset: AUDIO_DEVICE_SYSTEM,
    //     idDevice: 0,
    //     ePanningRule: rrise::AkPanningRule::AkPanningRule_Speakers,
    //     channelConfig: cc.as_ak(),
    // };

    let mut init = AkInitSettings {
        // settings_main_output,
        ..Default::default()
    };

    stream_mgr::set_current_language("English(US)").unwrap();
    sound_engine::init(&mut init, &mut AkPlatformInitSettings::default()).unwrap();
    music_engine::init(&mut settings::AkMusicSettings::default())?;

    Ok(())
}

fn term_sound_engine() -> Result<(), AkResult> {
    #[cfg(feature = "profiler")]
    profiling::scope!("term_sound_engine");

    music_engine::term();
    sound_engine::term();
    stream_mgr::term_tiger_stream_mgr();
    memory_mgr::term();

    Ok(())
}
