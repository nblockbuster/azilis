mod bank_list;
mod color;
mod icons;
pub mod player;
mod style;

use bank_list::BankListView;
use destiny_pkg::TagHash;
use eframe::egui::{self, Align2, Color32, CornerRadius, TextEdit, Vec2, Widget};
use egui_notify::Toasts;
use icons::ICON_STOP;
use lazy_static::lazy_static;
use player::{BankStatus, PlayerView, bank_progress};
use poll_promise::Promise;
use rrise::sound_engine::{
    clear_banks, set_game_object_output_bus_volume, unregister_all_game_obj,
};
use std::sync::{Arc, Mutex, atomic::Ordering};

use crate::{config, term_sound_engine};

lazy_static! {
    pub static ref TOASTS: Arc<Mutex<Toasts>> = Arc::new(Mutex::new(Toasts::new()));
}

#[derive(PartialEq)]
pub enum Panel {
    BankList,
    // Player,
}

pub enum ViewAction {
    OpenTag(TagHash),
}
pub trait View {
    fn view(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> Option<ViewAction>;
}

pub struct AzilisApp {
    // player_view: PlayerView,
    bank_list_view: BankListView,

    open_panel: Panel,
    tag_input: String,
    tag_split: bool,
    tag_split_input: (String, String),

    volume_control: f32,
}

impl AzilisApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "materialdesignicons".into(),
            Arc::new(egui::FontData::from_static(include_bytes!(
                "../../assets/fonts/materialdesignicons-webfont.ttf"
            ))),
        );
        fonts.font_data.insert(
            "Destiny_Keys".into(),
            egui::FontData::from_static(include_bytes!("../../assets/fonts/Destiny_Keys.otf"))
                .into(),
        );

        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "materialdesignicons".to_owned());
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(1, "Destiny_Keys".to_owned());

        cc.egui_ctx.set_fonts(fonts);
        set_game_object_output_bus_volume(100, 1, config!().audio.volume).unwrap();

        AzilisApp {
            // player_view: PlayerView::new(),
            bank_list_view: BankListView::new(),
            open_panel: Panel::BankList,
            volume_control: config!().audio.volume,
            tag_input: String::new(),
            tag_split: false,
            tag_split_input: (String::new(), String::new()),
        }
    }
}

impl eframe::App for AzilisApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut is_loading = false;
        if self.bank_list_view.player_view.bank_load.as_ref().is_some()
            || self.bank_list_view.list_load.as_ref().is_some()
        {
            let promise =
                if let Some(bank_promise) = self.bank_list_view.player_view.bank_load.as_ref() {
                    bank_promise
                } else if let Some(list_promise) = self.bank_list_view.list_load.as_ref() {
                    list_promise
                } else {
                    &Promise::spawn_thread("fake", Default::default)
                };

            if promise.poll().is_pending() {
                {
                    let painter = ctx.layer_painter(egui::LayerId::background());
                    painter.rect_filled(
                        egui::Rect::EVERYTHING,
                        CornerRadius::default(),
                        Color32::from_black_alpha(127),
                    );
                }
                egui::Window::new("Loading cache")
                    .collapsible(false)
                    .resizable(false)
                    .title_bar(false)
                    .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
                    .show(ctx, |ui| {
                        let progress = if let BankStatus::Externals {
                            current_file,
                            total_files,
                        } = bank_progress()
                        {
                            current_file as f32 / total_files as f32
                        } else {
                            0.9999
                        };

                        ui.add(
                            egui::ProgressBar::new(progress)
                                .animate(true)
                                .text(bank_progress().to_string()),
                        );
                    });
                is_loading = true;
            }
        }

        ctx.set_style(style::style());
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_enabled_ui(!is_loading, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Tag:");
                    let mut submitted = false;

                    if self.tag_split {
                        submitted |= TextEdit::singleline(&mut self.tag_split_input.0)
                            .hint_text("PKG ID")
                            .desired_width(64.)
                            .ui(ui)
                            .lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));

                        submitted |= TextEdit::singleline(&mut self.tag_split_input.1)
                            .hint_text("Index")
                            .desired_width(64.)
                            .ui(ui)
                            .lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    } else {
                        submitted |= TextEdit::singleline(&mut self.tag_input)
                            .hint_text("32-bit tag/Bank ID")
                            .desired_width(128. + 8.)
                            .ui(ui)
                            .lost_focus()
                            && ui.input(|i| i.key_pressed(egui::Key::Enter));
                    }

                    if ui.button("Open").clicked() || submitted {
                        let tag_input_trimmed = self.tag_input.trim();
                        let tag = if self.tag_split {
                            let pkg_id = self.tag_split_input.0.trim();
                            let entry_index = self.tag_split_input.1.trim();

                            if pkg_id.is_empty() || entry_index.is_empty() {
                                TagHash::NONE
                            } else {
                                let pkg_id: u16 =
                                    u16::from_str_radix(pkg_id, 16).unwrap_or_default();
                                let entry_index = str::parse(entry_index).unwrap_or_default();
                                TagHash::new(pkg_id, entry_index)
                            }
                        } else if tag_input_trimmed.len() > 8
                            && tag_input_trimmed.chars().all(char::is_numeric)
                        {
                            let hash = tag_input_trimmed.parse().unwrap_or_default();
                            TagHash(hash)
                        } else if !tag_input_trimmed.is_empty() {
                            let hash =
                                u32::from_str_radix(tag_input_trimmed, 16).unwrap_or_default();
                            TagHash(u32::from_be(hash))
                        } else {
                            TagHash::NONE
                        };

                        self.open_bank(tag);
                    }

                    ui.checkbox(&mut self.tag_split, "Split pkg/entry");

                    if ui
                        .add(
                            egui::Slider::new(&mut self.volume_control, 0.000..=1.0).text("Volume"),
                        )
                        .changed()
                    {
                        set_game_object_output_bus_volume(100, 1, self.volume_control).unwrap();
                        config::with_mut(|c| c.audio.volume = self.volume_control);
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    ui.selectable_value(&mut self.open_panel, Panel::BankList, "Bank List");
                    // ui.selectable_value(&mut self.open_panel, Panel::Player, "Player");
                });
                ui.separator();
                let action = match self.open_panel {
                    // Panel::Player => self.player_view.view(ctx, ui),
                    Panel::BankList => self.bank_list_view.view(ctx, ui),
                    // TODO: Standalone Hierachy View
                    _ => None,
                };

                if let Some(action) = action {
                    match action {
                        ViewAction::OpenTag(t) => self.open_bank(t),
                    }
                }
            });
        });
        TOASTS.lock().unwrap().show(ctx);
    }
}

impl AzilisApp {
    fn open_bank(&mut self, tag: TagHash) {
        if tag.is_none() {
            return;
        }
        let loaded_bank = self.bank_list_view.player_view.bank_data.lock().unwrap().id;
        if loaded_bank != 0 {
            let bnk_ptr = self
                .bank_list_view
                .player_view
                .bank_data
                .lock()
                .unwrap()
                .bank_data[0]
                .as_mut_ptr() as *mut _;
            rrise::sound_engine::unload_bank_by_id(loaded_bank, bnk_ptr).unwrap();
        }
        let new_view = PlayerView::create(tag);
        self.bank_list_view.player_view.stop();
        self.bank_list_view.player_view = new_view;
        // self.open_panel = Panel::Player;
    }
}

impl Drop for AzilisApp {
    fn drop(&mut self) {
        self.bank_list_view.player_view.stop();
        clear_banks().unwrap();
        unregister_all_game_obj().unwrap();
        term_sound_engine().unwrap();
        config::persist();
    }
}
