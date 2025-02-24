use destiny_pkg::TagHash;
use destiny_pkg::manager::PackagePath;
use destiny_pkg::package::UEntryHeader;
use eframe::egui::{self, Context, RichText, Ui};
use itertools::Itertools;
use log::warn;
use parser::{SoundbankChunkTypes, hierarchy::music::MusicSwitchContainer};
use poll_promise::Promise;
use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
use std::sync::{Arc, Mutex};

use crate::package_manager;
use crate::util::format_file_size;

use super::player::{BankData, PlayerView};
use super::{View, ViewAction};

pub struct BankListView {
    selected_package: u16,
    package_entry_search_cache: Vec<(usize, String, UEntryHeader)>,
    package_filter: String,
    package_entry_filter: String,
    sorted_package_paths: Vec<(u16, PackagePath)>,
    valid_music_banks: Arc<Mutex<Vec<TagHash>>>,

    pub list_load: Option<Promise<BankData>>,

    pub player_view: PlayerView,
    // pub player_open: bool,
}

impl BankListView {
    pub fn new() -> Self {
        let mut sorted_package_paths: Vec<(u16, PackagePath)> = package_manager()
            .package_paths
            .iter()
            .map(|(id, path)| (*id, path.clone()))
            .collect();

        sorted_package_paths.sort_by_cached_key(|(_, path)| format!("{}_{}", path.name, path.id));
        let valid_music_banks: Arc<Mutex<Vec<TagHash>>> = Default::default();

        let mus_banks = valid_music_banks.clone();
        Self {
            selected_package: u16::MAX,
            package_entry_search_cache: vec![],
            package_filter: String::new(),
            package_entry_filter: String::new(),
            sorted_package_paths,
            valid_music_banks,
            list_load: Some(Promise::spawn_thread("load_pkglist", move || {
                let all_banks = package_manager().get_all_by_type(26, Some(6));
                let valid_hashes = mus_banks.clone();
                all_banks.par_iter().for_each(|(th, _)| {
                    let data = package_manager().read_tag(*th);
                    if let Some(e) = data.as_ref().err() {
                        warn!("{}", e);
                        return;
                    }
                    let data = data.unwrap();
                    let chunks = parser::parse(&data);
                    // if let Some(e) = chunks.as_ref().err() {
                    if chunks.as_ref().is_err() {
                        return;
                    }
                    let mut chunks = chunks.unwrap();
                    let hirc = &mut chunks
                        .iter_mut()
                        .find_map(|c| {
                            if let SoundbankChunkTypes::Hierarchy(hirc) = &c.chunk {
                                return Some(hirc.clone());
                            };
                            None
                        })
                        .unwrap();
                    if !hirc.get_all_by_type::<MusicSwitchContainer>().is_empty() {
                        valid_hashes.lock().unwrap().push(*th);
                    }
                });

                BankData::default()
            })),
            player_view: PlayerView::new(),
        }
    }
}
impl View for BankListView {
    fn view(&mut self, ctx: &Context, ui: &mut Ui) -> Option<ViewAction> {
        egui::SidePanel::left("packages_left_panel")
            .min_width(256.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                ui.horizontal(|ui| {
                    ui.label("Search:");
                    ui.text_edit_singleline(&mut self.package_filter);
                });
                egui::ScrollArea::vertical()
                    .max_width(f32::INFINITY)
                    .show(ui, |ui| {
                        for (id, path) in self.sorted_package_paths.iter() {
                            let package_name = format!("{}_{}", path.name, path.id);
                            if !self.package_filter.is_empty()
                                && !package_name
                                    .to_lowercase()
                                    .contains(&self.package_filter.to_lowercase())
                            {
                                continue;
                            }

                            if !self
                                .valid_music_banks
                                .lock()
                                .unwrap()
                                .iter()
                                .map(|x| x.pkg_id())
                                .collect_vec()
                                .contains(id)
                            {
                                continue;
                            }

                            let redacted = if path.name.ends_with("redacted") {
                                "🗝 "
                            } else {
                                ""
                            };

                            if ui
                                .selectable_value(
                                    &mut self.selected_package,
                                    *id,
                                    format!("{id:04x}: {redacted}{package_name}"),
                                )
                                .changed()
                            {
                                self.package_entry_search_cache = vec![];
                                if let Ok(p) = package_manager().version.open(&path.path) {
                                    for (i, e) in p.entries().iter().enumerate() {
                                        if e.file_type != 26 && e.file_subtype != 6 {
                                            continue;
                                        }
                                        let label = TagHash::new(*id, i as u16).to_string();

                                        self.package_entry_search_cache.push((i, label, e.clone()));
                                    }
                                }
                            }
                        }
                    });
            });

        let ret = egui::SidePanel::left("banks_left_panel")
            .min_width(280.0)
            .resizable(true)
            .show_inside(ui, |ui| {
                if self.selected_package == u16::MAX {
                    ui.label(RichText::new("No package selected").italics());

                    None
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Search:");
                        ui.text_edit_singleline(&mut self.package_entry_filter);
                    });

                    egui::ScrollArea::vertical()
                        .max_width(f32::INFINITY)
                        .id_salt("package_tags")
                        .show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);

                            for (i, (tag, label, entry)) in self
                                .package_entry_search_cache
                                .iter()
                                .enumerate()
                                .filter(|(_, (_, label, _))| {
                                    self.package_entry_filter.is_empty()
                                        || label
                                            .to_lowercase()
                                            .contains(&self.package_entry_filter.to_lowercase())
                                })
                                .map(|(_, (i, label, entry))| {
                                    let tag = TagHash::new(self.selected_package, *i as u16);
                                    (i, (tag, label.clone(), entry))
                                })
                            {
                                if !self.valid_music_banks.lock().unwrap().contains(&tag) {
                                    continue;
                                }
                                ctx.style_mut(|s| {
                                    s.interaction.show_tooltips_only_when_still = false;
                                    s.interaction.tooltip_delay = 0.0;
                                });
                                if ui
                                    .add(egui::SelectableLabel::new(
                                        false,
                                        RichText::new(format!(
                                            "{i}: {label} ({})",
                                            format_file_size(entry.file_size as usize)
                                        )),
                                    ))
                                    .clicked()
                                {
                                    return Some(ViewAction::OpenTag(tag));
                                }
                            }

                            None
                        })
                        .inner
                }
            })
            .inner;

        egui::CentralPanel::default().show_inside(ui, |ui| {
            self.player_view.view(ctx, ui);
        });

        ret
    }
}
