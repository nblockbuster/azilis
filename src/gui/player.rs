use destiny_pkg::TagHash;
use eframe::egui::{self, Align2, Color32, Context, CornerRadius, Ui, Vec2};
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
use rrise::sound_engine::clear_banks;
use rrise::{
    AkCodecId, game_syncs,
    sound_engine::{AkExternalSourceInfo, PostEvent, load_bank_memory_copy, render_audio},
    stream_mgr,
};
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

use super::{TOASTS, View, ViewAction, icons::*};

const MUSIC_GROUP_ID: u32 = 1246133352;

#[derive(Default)]
pub struct BankData {
    loaded_banks: Vec<u32>,
    play_event_id: u32,
    stop_event_id: u32,
    main_switch: MusicSwitchContainer,
    // tracks: Vec<MusicTrack>,
    externals: Vec<AkExternalSourceInfo>,
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
                "Checking/Writing Files {}/{}",
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

pub struct PlayerView {
    tag: TagHash,
    tag_data: Vec<u8>,

    pub bank_load: Option<Promise<BankData>>,
    bank_data: Arc<Mutex<BankData>>,

    current_switch_id: Arc<AtomicU32>,

    audio_thread: Option<JoinHandle<()>>,

    switch_dropdown: String,
    apply_dropdown: bool,
}

impl PlayerView {
    pub fn new() -> Self {
        Self {
            tag: TagHash::NONE,
            tag_data: Vec::new(),

            bank_load: None,
            bank_data: Default::default(),

            current_switch_id: Arc::new(AtomicU32::new(0)),
            audio_thread: Default::default(),
            switch_dropdown: String::new(),
            apply_dropdown: false,
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

        Self {
            tag,
            tag_data: tag_data.clone(),

            bank_load: Some(Promise::spawn_thread("load_bank", move || {
                let bnk = load_bank(&mut tag_data.clone());
                if let Some(e) = bnk.as_ref().err() {
                    TOASTS.lock().unwrap().error(format!("{:?}", e));
                    return BankData::default();
                }

                bnk.unwrap()
            })),
            bank_data: Default::default(),
            current_switch_id,
            audio_thread: Some(std::thread::spawn(move || {
                let mut last_switch: u32 = 0;
                loop {
                    let switch = switch_id.load(Ordering::Relaxed);
                    if switch != last_switch {
                        game_syncs::set_switch(MUSIC_GROUP_ID, switch, 100).unwrap();
                    }
                    last_switch = switch;
                    const ALLOW_SYNC_RENDER: bool = true;
                    render_audio(ALLOW_SYNC_RENDER).unwrap();

                    // if should_stop.load(Ordering::SeqCst) {
                    //     info!("Stopping loop");
                    //     stop_all(None);
                    //     unregister_all_game_obj().unwrap();
                    //     clear_banks().unwrap();
                    //     break;
                    // }
                }
            })),
            switch_dropdown: String::new(),
            apply_dropdown: false,
            // loaded_banks: Vec::new(),
            // play_event_id: play_event.id,
            // main_switch_id: main_switch.id,
            // tracks,
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

        ui.horizontal(|ui| {
            if self.tag.is_some() {
                ui.label(self.tag.to_string());
            }
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
        });

        if self.apply_dropdown {
            info!("Setting switch to {}", self.switch_dropdown);
            self.current_switch_id.store(
                self.switch_dropdown.parse::<u32>().unwrap(),
                Ordering::Relaxed,
            );
            self.apply_dropdown = false;
        }

        if change_event {
            if let Ok(playing_id) = PostEvent::new(100, id, data.externals.to_vec()).post() {
                info!("Successfully started event with playingID {}", playing_id);
            } else {
                panic!("Couldn't post event");
            }
        }
        None
    }
}

fn load_bank(data: &mut [u8]) -> anyhow::Result<BankData> {
    // rayon::ThreadPoolBuilder::new()
    //     .thread_name(|i| format!("rayon-bank-load-worker-{i}"))
    //     .num_threads(4)
    //     .build_global()
    //     .unwrap();

    clear_banks()?;
    *BANK_PROGRESS.write() = BankStatus::LoadingBanks;
    let mut loaded_banks = Vec::new();
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("load banks from pkg");

        let init_tags = package_manager().get_all_by_type(26, Some(5));

        {
            let mut init_data = package_manager().read_tag(init_tags.first().unwrap().0)?;
            let data_len = init_data.len() as u32;
            let id =
                load_bank_memory_copy(init_data.as_mut_ptr() as *mut std::ffi::c_void, data_len)?;
            loaded_banks.push(id);
        }
    }
    let mut soundbank_sections = {
        #[cfg(feature = "profiler")]
        profiling::scope!("soundbank parse");
        let data_len = data.len() as u32;
        let id = load_bank_memory_copy(data.as_mut_ptr() as *mut std::ffi::c_void, data_len)?;
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

    let stop_action = stop_actions
        .iter()
        .filter(|x| x.object_id == main_switch.id)
        .collect_vec()[0];

    let stop_event = &hirc.filter_objects(|x: &Event| x.action_ids.contains(&stop_action.id))[0];

    let tmpdir = std::env::temp_dir().join("azilis");
    std::fs::create_dir_all(&tmpdir)?;
    stream_mgr::add_base_path(tmpdir.to_str().unwrap())?;

    // *BANK_PROGRESS.write() = BankStatus::Externals { current_file: (), total_files: () };

    let externals = Arc::new(Mutex::new(Vec::new()));
    {
        #[cfg(feature = "profiler")]
        profiling::scope!("load externals");

        // TODO: speed
        tracks
            .clone()
            .par_iter_mut()
            .try_for_each(|x| -> anyhow::Result<()> {
                {
                    let mut p = BANK_PROGRESS.write();
                    let current_file = if let BankStatus::Externals { current_file, .. } = *p {
                        current_file
                    } else {
                        0
                    };

                    *p = BankStatus::Externals {
                        current_file: current_file + 1,
                        total_files: tracks.len(),
                    };
                }

                let th = package_manager().get_all_by_reference(x.sounds[0].audio_id)[0].0;
                let head = package_manager().get_entry(th).unwrap();
                let path = tmpdir.join(format!("{}.wem", head.reference));
                if path.exists() {
                    externals
                        .lock()
                        .unwrap()
                        .push(AkExternalSourceInfo::from_id(
                            head.reference,
                            head.reference,
                            AkCodecId::Vorbis,
                        ));

                    return Ok(());
                }
                trace!("Loading {:?}.wem", head.reference);
                let data = package_manager().read_tag(th)?;
                trace!("Writing {:?}.wem", head.reference);

                let mut file = std::fs::File::create(&path).unwrap();
                file.write_all(&data).unwrap();

                externals
                    .lock()
                    .unwrap()
                    .push(AkExternalSourceInfo::from_id(
                        head.reference,
                        head.reference,
                        AkCodecId::Vorbis,
                    ));
                Ok(())
            })
            .unwrap();
        // pb.finish();
    }
    let mut externals = externals.lock().unwrap();
    externals.dedup_by(|a, b| a.external_src_cookie == b.external_src_cookie);

    info!("loaded {} banks", loaded_banks.len());
    info!("loaded {} externals", externals.len());
    Ok(BankData {
        externals: externals.to_vec(),
        loaded_banks,
        play_event_id: play_event.id,
        stop_event_id: stop_event.id,
        main_switch: main_switch.clone(),
        // tracks,
    })
}
