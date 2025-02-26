use chroma_dbg::ChromaDebug;
use destiny_pkg::TagHash;
use eframe::egui::ahash::HashMap;
use eframe::egui::{Color32, Context, FontId, RichText, Ui};
use eframe::epaint::mutex::RwLock;
use egui_dropdown::DropDownBox;
use itertools::Itertools;
use log::{info, trace};
use parser::{
    SoundbankChunkTypes,
    hierarchy::{
        event::{Event, EventAction, EventActionType},
        music::{AudioPathElement, MusicSwitchContainer, MusicTrack},
    },
};
use poll_promise::Promise;
use rayon::iter::{IntoParallelRefMutIterator, ParallelIterator};
use rrise::sound_engine::{clear_banks, load_bank_memory_view, stop_all, unregister_all_game_obj};
use rrise::{AkCallbackInfo, AkCallbackType};
use rrise::{
    AkCodecId, game_syncs,
    sound_engine::{PostEvent, load_bank_memory_copy, render_audio},
    stream_mgr,
};
use std::sync::atomic::AtomicBool;
use std::thread::JoinHandle;
use std::{
    fmt::Display,
    io::Write,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU32, Ordering},
    },
};

use crate::package_manager;

use super::{TOASTS, View, ViewAction, color, icons::*, style};

pub const MUSIC_GROUP_ID: u32 = 1246133352;

#[derive(Default, Debug)]
pub struct BankData {
    pub id: u32,
    // loaded_banks: Vec<u32>,
    pub play_event_id: u32,
    pub stop_event_id: u32,
    pub main_switch: MusicSwitchContainer,
    // tracks: Vec<MusicTrack>,
    // pub externals: Vec<AkExternalSourceInfo>,
    pub bank_data: Vec<Vec<u8>>,
}

#[derive(Copy, Clone)]
pub enum BankStatus {
    None,
    LoadingBanks,
    ReadingHierarchy,
    Externals {
        current_file: usize,
        total_files: usize,
    },
}

impl Display for BankStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BankStatus::None => Ok(()),
            BankStatus::LoadingBanks => f.write_str("Loading Soundbanks"),
            BankStatus::ReadingHierarchy => f.write_str("Reading Bank Hierarchy"),
            BankStatus::Externals {
                current_file,
                total_files,
            } => f.write_fmt(format_args!(
                "Loading Externals {}/{}",
                current_file, total_files
            )),
        }
    }
}

lazy_static::lazy_static! {
    static ref BANK_PROGRESS: RwLock<BankStatus> = RwLock::new(BankStatus::None);
}

pub fn bank_progress() -> BankStatus {
    *BANK_PROGRESS.read()
}

#[derive(Copy, Clone, Debug, Eq, Hash, PartialEq)]
pub enum CallbackType {
    MusicPlaylist,
    MusicSyncBeat,
    MusicSyncBar,
    MusicSyncEntry,
    MusicSyncExit,
    MusicSyncPoint,
    Duration,
}

pub struct PlayerView {
    tag: TagHash,
    tag_data: Vec<u8>,

    pub bank_load: Option<Promise<BankData>>,
    pub bank_data: Arc<Mutex<BankData>>,

    current_switch_id: Arc<AtomicU32>,

    stop_audio: Arc<AtomicBool>,
    audio_thread: Option<JoinHandle<()>>,

    switch_dropdown: String,
    apply_dropdown: bool,

    callback_infos: Arc<RwLock<HashMap<CallbackType, AkCallbackInfo>>>,
    // pub loaded_bank: u32,
    // pub old_bank: u32,
}

impl PlayerView {
    pub fn stop(&mut self) {
        self.stop_audio.store(true, Ordering::Relaxed);
        // self.callback_infos.write().clear();
        // self.tag_data.clear();
        // let a = self.bank_data.clone();
        // let mut b = a.lock().unwrap();
        // b.externals.clear();
    }

    pub fn new() -> Self {
        Self {
            tag: TagHash::NONE,
            tag_data: Vec::new(),

            bank_load: None,
            bank_data: Default::default(),

            current_switch_id: Arc::new(AtomicU32::new(0)),
            stop_audio: Arc::new(AtomicBool::new(false)),
            audio_thread: Default::default(),
            switch_dropdown: String::new(),
            apply_dropdown: false,
            callback_infos: Default::default(),
        }
    }

    pub fn create(tag: TagHash) -> Self {
        let t = package_manager().read_tag(tag);
        let tag_data = if t.is_err() {
            let real_tags = package_manager().get_all_by_reference(tag.0);
            let real_tag = real_tags.first().unwrap();
            package_manager().read_tag(real_tag.0).ok().unwrap()
        } else {
            package_manager().read_tag(tag).ok().unwrap()
        };

        let current_switch_id = Arc::new(AtomicU32::new(0));
        let switch_id = current_switch_id.clone();

        let stop_audio = Arc::new(AtomicBool::new(false));
        let should_stop_audio = stop_audio.clone();

        Self {
            tag,
            tag_data: tag_data.clone(),

            bank_load: Some(Promise::spawn_thread("load_bank", move || {
                let bnk = load_bank(&mut tag_data.clone());
                if let Some(e) = bnk.as_ref().err() {
                    TOASTS
                        .lock()
                        .unwrap()
                        .error(format!("{:?}", e.root_cause()));
                    return BankData::default();
                }
                let bnk = bnk.unwrap();
                // info!("{}", bnk.externals.dbg_chroma());
                bnk
            })),
            bank_data: Default::default(),
            current_switch_id,
            stop_audio,
            audio_thread: Some(std::thread::spawn(|| {
                Self::audio_thread(should_stop_audio, switch_id)
            })),
            switch_dropdown: String::new(),
            apply_dropdown: false,
            callback_infos: Default::default(),
        }
    }

    fn audio_thread(should_stop_audio: Arc<AtomicBool>, switch_id: Arc<AtomicU32>) {
        #[cfg(feature = "profiler")]
        profiling::register_thread!("rust_audio_thread");
        let mut last_switch: u32 = 0;
        loop {
            if should_stop_audio.load(Ordering::Relaxed) {
                stop_all(Some(100));
                break;
            }

            let switch = switch_id.load(Ordering::Relaxed);
            if switch != last_switch {
                #[cfg(feature = "profiler")]
                profiling::scope!("set_switch");
                game_syncs::set_switch(MUSIC_GROUP_ID, switch, 100).unwrap();
            }
            last_switch = switch;
            // #[cfg(feature = "profiler")]
            // profiling::scope!("render_audio");
            render_audio(true).unwrap();
        }
    }
}
impl View for PlayerView {
    fn view(&mut self, ctx: &Context, ui: &mut Ui) -> Option<ViewAction> {
        if self
            .bank_load
            .as_ref()
            .map(|v| v.poll().is_ready())
            .unwrap_or_default()
        {
            let c = self.bank_load.take().unwrap();
            let bnk_data = c.try_take().unwrap_or_default();
            self.bank_data = Arc::new(Mutex::new(bnk_data));

            let sw = self.bank_data.lock().unwrap().main_switch.clone();
            let first_switch =
                if let Some(AudioPathElement::MusicEndpoint(a)) = &sw.paths.children.first() {
                    a.from_id
                } else {
                    0
                };

            self.switch_dropdown = format!("{}", first_switch);

            self.current_switch_id
                .store(first_switch, Ordering::Relaxed);

            ctx.request_repaint();
        }

        let data = self.bank_data.clone();
        let data = data.lock().unwrap();

        let mut change_event = false;
        let mut id = 0;

        let bar_resp = ui
            .horizontal(|ui| {
                if self.tag.is_some() && ui.label(self.tag.to_string()).secondary_clicked() {
                    ctx.copy_text(self.tag.to_string());
                }
                // TODO: Up/Down arrows
                ui.add(
                    DropDownBox::from_iter(
                        data.main_switch.paths.children.iter().map(|x| {
                            if let AudioPathElement::MusicEndpoint(m) = x {
                                return format!("{}", m.from_id);
                            }
                            String::new()
                        }),
                        "Switch IDs",
                        &mut self.switch_dropdown,
                        |ui, text| ui.selectable_label(false, text),
                    )
                    .max_height(ctx.available_rect().height() * 0.5)
                    .filter_by_input(false),
                );

                if ui.button(format!("{}", ICON_CHECK)).clicked() {
                    // self.switch_dropdown = cur_dropdown.clone();
                    self.apply_dropdown = true;
                };

                ui.separator();

                id = if ui.button(format!("{} Play", ICON_PLAY)).clicked() {
                    change_event = true;
                    data.play_event_id
                } else if ui.button(format!("{} Stop", ICON_STOP)).clicked() {
                    change_event = true;
                    data.stop_event_id
                } else {
                    change_event = false;
                    0
                };
            })
            .response;

        if self.apply_dropdown {
            self.apply_dropdown = false;
            info!("Setting switch to {}", self.switch_dropdown);
            let dropdown_val = self.switch_dropdown.parse::<u32>();
            if dropdown_val.is_err() {
                TOASTS.lock().unwrap().error("Could not parse switch ID");
                return None;
            }
            self.current_switch_id
                .store(dropdown_val.unwrap(), Ordering::Relaxed);
        }
        let infos = self.callback_infos.clone();
        if change_event {
            if let Ok(playing_id) = PostEvent::new(100, id)
                .add_flags(AkCallbackType::AK_MusicPlayStarted)
                .add_flags(AkCallbackType::AK_MusicPlaylistSelect)
                .add_flags(AkCallbackType::AK_MusicSyncAll)
                .add_flags(AkCallbackType::AK_Duration)
                .post_with_callback(move |info| {
                    #[cfg(feature = "profiler")]
                    profiling::register_thread!("wwise-audio-thread");
                    #[cfg(feature = "profiler")]
                    profiling::scope!("callback");
                    match info {
                        AkCallbackInfo::MusicSync {
                            music_sync_type, ..
                        } => match music_sync_type {
                            AkCallbackType::AK_MusicSyncBar => {
                                infos.write().insert(CallbackType::MusicSyncBar, info);
                            }
                            AkCallbackType::AK_MusicSyncBeat => {
                                infos.write().insert(CallbackType::MusicSyncBeat, info);
                            }
                            // AkCallbackType::AK_MusicSyncEntry => {
                            //     infos.write().insert(CallbackType::MusicSyncEntry, info);
                            // }
                            // AkCallbackType::AK_MusicSyncExit => {
                            //     infos.write().insert(CallbackType::MusicSyncExit, info);
                            // }
                            // AkCallbackType::AK_MusicSyncPoint => {
                            //     infos.write().insert(CallbackType::MusicSyncPoint, info);
                            // }
                            // AkCallbackType::AK_Duration => {
                            //     infos.write().insert(CallbackType::Duration, info);
                            // }
                            _ => {}
                        },
                        AkCallbackInfo::MusicPlaylist { .. } => {
                            infos.write().insert(CallbackType::MusicPlaylist, info);
                        }
                        _ => {}
                    }
                })
            {
                info!("Successfully started event with playingID {}", playing_id);
            } else {
                panic!("Couldn't post event");
            }
        }

        if !self.callback_infos.read().is_empty() {
            eframe::egui::ScrollArea::vertical()
                .max_height(ctx.available_rect().height() * 0.9)
                .max_width(bar_resp.rect.width())
                .auto_shrink([false, false])
                .id_salt("bank_music_syncs")
                .show(ui, |ui| {
                    ui.separator();

                    ui.label(
                        RichText::new(format!("Switch State ID: {}", self.current_switch_id.load(Ordering::Relaxed)))
                            .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                    );

                    if let Some(playlist_callback) =
                        self.callback_infos.read().get(&CallbackType::MusicPlaylist)
                        && let AkCallbackInfo::MusicPlaylist {
                            playlist_id,
                            num_playlist_items,
                            playlist_selection,
                            ..
                        } = playlist_callback
                    {
                        {   
                            ui.label(
                                RichText::new(format!("Playlist ID: {}", playlist_id,))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            );
                            ui.label(
                                RichText::new(format!("Playlist Items: {}", num_playlist_items))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            );
                            ui.label(
                                RichText::new(format!("Selected Item: {}", playlist_selection))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            );
                        }
                    }

                    ui.separator();
                    ui.label(RichText::new("Music Syncs").font(FontId::proportional(style::TEXT_HEADER_SIZE)));
                    ui.separator();

                    for (callback_type, info) in self.callback_infos.read().iter() {
                        if let AkCallbackInfo::MusicSync { segment_info, .. } = info {
                            ui.label(
                                RichText::new(format!("{:#?}", callback_type))
                                    .font(FontId::proportional(style::TEXT_SUBHEADER_SIZE)),
                            );
                            ui.label(
                                RichText::new(format!("\tCurrent Position: {}", segment_info.iCurrentPosition))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Current position of the segment, relative to the Entry Cue, in milliseconds. Range is -iPreEntryDuration, iActiveDuration+iPostExitDuration");
                            ui.label(
                                RichText::new(format!("\tPre Entry Duration: {}", segment_info.iPreEntryDuration))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Duration of the pre-entry region of the segment, in milliseconds.");
                            ui.label(
                                RichText::new(format!("\tActive Duration: {}", segment_info.iActiveDuration))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Duration of the active region of the segment (between the Entry and Exit Cues), in milliseconds.");
                            ui.label(
                                RichText::new(format!("\tPost Exit Duration: {}", segment_info.iPostExitDuration))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Duration of the post-exit region of the segment, in milliseconds.");
                            ui.label(
                                RichText::new(format!("\tRemaining Look Ahead Time: {}", segment_info.iRemainingLookAheadTime))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Number of milliseconds remaining in the \"looking-ahead\" state of the segment, when it is silent but streamed tracks are being prefetched.");
                            ui.label(
                                RichText::new(format!("\tBeat Duration: {} ({}bpm)", segment_info.fBeatDuration, (60.0/segment_info.fBeatDuration).floor()))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Beat Duration in seconds.");
                            ui.label(
                                RichText::new(format!("\tBar Duration: {}", segment_info.fBarDuration))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Bar Duration in seconds.");
                            ui.label(
                                RichText::new(format!("\tGrid Duration: {}", segment_info.fGridDuration))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Grid duration in seconds.");
                            ui.label(
                                RichText::new(format!("\tGrid Offset: {}", segment_info.fGridOffset))
                                    .font(FontId::proportional(style::TEXT_BODY_SIZE)),
                            ).on_hover_text("Grid offset in seconds.");
                            ui.separator();
                        }
                    }
                    ctx.request_repaint();
                });
        }

        None
    }
}

pub fn load_bank(data: &mut [u8]) -> anyhow::Result<BankData> {
    // clear_banks()?;
    *BANK_PROGRESS.write() = BankStatus::LoadingBanks;
    let mut loaded_banks = Vec::new();
    let mut bank_data = Vec::new();
    // {
    //     #[cfg(feature = "profiler")]
    //     profiling::scope!("load banks from pkg");

    //     let init_tags = package_manager().get_all_by_type(26, Some(5));

    //     {
    //         let init_data = package_manager().read_tag(init_tags.first().unwrap().0)?;
    //         let data_len = init_data.len() as u32;
    //         bank_data.push(init_data);
    //         let id = load_bank_memory_view(
    //             bank_data[0].as_mut_ptr() as *mut std::ffi::c_void,
    //             data_len,
    //         )?;
    //         loaded_banks.push(id);
    //     }
    // }
    let mut soundbank_sections = {
        #[cfg(feature = "profiler")]
        profiling::scope!("soundbank parse");
        let data_len = data.len() as u32;
        bank_data.push(data.to_vec());
        let id = load_bank_memory_view(bank_data[0].as_mut_ptr() as *mut _, data_len)?;
        loaded_banks.push(id);

        parser::parse(data)?
    };

    *BANK_PROGRESS.write() = BankStatus::ReadingHierarchy;

    let hirc = &mut soundbank_sections
        .iter_mut()
        .find_map(|c| {
            if let SoundbankChunkTypes::Hierarchy(hirc) = &c.chunk {
                return Some(hirc.clone());
            };
            None
        })
        .unwrap();

    // std::fs::write("temp/hirc.txt", format!("{:#?}", hirc))?;

    let tracks: Vec<MusicTrack> = hirc.get_all_by_type_cloned();

    let play_actions =
        &hirc.filter_objects(|x: &EventAction| x.action_type == EventActionType::Play);

    let switches = hirc.filter_objects(|x: &MusicSwitchContainer| {
        for a in play_actions {
            if x.id == a.object_id {
                return true;
            }
        }
        false
    });
    let main_switch = switches.first();
    if main_switch.is_none() {
        return Err(anyhow::anyhow!(
            "No MusicSwitchContainer objects found in the hierarchy"
        ));
    }
    let main_switch = main_switch.unwrap();

    let play_action = play_actions
        .iter()
        .filter(|x| x.object_id == main_switch.id)
        .collect_vec()[0];

    let play_event = &hirc.filter_objects(|x: &Event| x.action_ids.contains(&play_action.id))[0];

    let stop_actions =
        &hirc.filter_objects(|x: &EventAction| x.action_type == EventActionType::Stop);

    let matching_stops = stop_actions
        .iter()
        .filter(|x| x.object_id == main_switch.id)
        .collect_vec();

    let stop_action = matching_stops.first();

    if stop_action.is_none() {
        return Err(anyhow::anyhow!(
            "No MusicSwitchContainer objects found in the hierarchy"
        ));
    }
    let stop_action = stop_action.unwrap();

    let stop_event = &hirc.filter_objects(|x: &Event| x.action_ids.contains(&stop_action.id))[0];

    // let tmpdir = std::env::temp_dir().join("azilis");
    // std::fs::create_dir_all(&tmpdir)?;
    // stream_mgr::add_base_path(tmpdir.to_str().unwrap())?;

    // *BANK_PROGRESS.write() = BankStatus::Externals { current_file: (), total_files: () };

    // let externals = Arc::new(Mutex::new(Vec::new()));
    // {
    //     #[cfg(feature = "profiler")]
    //     profiling::scope!("load externals");

    //     // TODO: speed
    //     tracks.clone().par_iter_mut().for_each(|x| {
    //         {
    //             let mut p = BANK_PROGRESS.write();
    //             let current_file = if let BankStatus::Externals { current_file, .. } = *p {
    //                 current_file
    //             } else {
    //                 0
    //             };

    //             *p = BankStatus::Externals {
    //                 current_file: current_file + 1,
    //                 total_files: tracks.len(),
    //             };
    //         }
    //         if x.sounds.is_empty() {
    //             return;
    //             // return Err(anyhow::anyhow!("No sounds found"));
    //         }
    //         let th = package_manager().get_all_by_reference(x.sounds[0].audio_id)[0].0;
    //         let head = package_manager().get_entry(th).unwrap();
    //         // let path = tmpdir.join(format!("{}.wem", head.reference));
    //         // if path.exists() {
    //         //     externals
    //         //         .lock()
    //         //         .unwrap()
    //         //         .push(AkExternalSourceInfo::from_id(
    //         //             head.reference,
    //         //             head.reference,
    //         //             AkCodecId::Vorbis,
    //         //         ));

    //         //     return Ok(());
    //         // }
    //         // trace!("Loading {:?}.wem", head.reference);
    //         // let data = package_manager().read_tag(th)?;
    //         // trace!("Writing {:?}.wem", head.reference);

    //         // let mut file = std::fs::File::create(&path).unwrap();
    //         // file.write_all(&data).unwrap();

    //         externals
    //             .lock()
    //             .unwrap()
    //             .push(AkExternalSourceInfo::from_id(
    //                 head.reference,
    //                 head.reference,
    //                 AkCodecId::Vorbis,
    //             ));
    //     });
    //     // pb.finish();
    // }
    // let mut externals = externals.lock().unwrap();
    // externals.dedup_by(|a, b| a.external_src_cookie == b.external_src_cookie);

    info!("loaded {} banks", loaded_banks.len());
    // info!("loaded {} externals", externals.len());
    Ok(BankData {
        id: loaded_banks[0],
        // externals: externals.to_vec(),
        play_event_id: play_event.id,
        stop_event_id: stop_event.id,
        main_switch: main_switch.clone(),
        bank_data,
    })
}
