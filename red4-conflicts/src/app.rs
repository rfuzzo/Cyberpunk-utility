use std::{collections::HashMap, env};

use egui::Color32;
use red4lib::{fnv1a64_hash_path, get_red4_hashes};

use crate::{ArchiveViewModel, ETheme, ETooltipVisuals, TemplateApp};

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // first time, load hashes
        if self.hashes.is_empty() {
            self.hashes = get_red4_hashes();
        }
        // first time, set game path to cwd
        if !self.game_path.exists() {
            if let Ok(current_dir) = env::current_dir() {
                self.game_path = current_dir;
            }
        }
        // set dark mode by default
        if self.theme.is_none() {
            ctx.set_visuals(egui::Visuals::dark())
        } else if let Some(theme) = &self.theme {
            match theme {
                crate::ETheme::Dark => ctx.set_visuals(egui::Visuals::dark()),
                crate::ETheme::Light => ctx.set_visuals(egui::Visuals::light()),
            }
        }

        // auto-generate hashes on first load and load order change
        if let Some(last_load_order) = &self.last_load_order {
            if &self.load_order != last_load_order {
                self.generate_conflict_map();
                self.last_load_order = Some(self.load_order.clone());
                // serialize to modlist.txt
                self.serialize_load_order();
            }
        } else {
            self.generate_conflict_map();
            self.last_load_order = Some(self.load_order.clone());
            // serialize to modlist.txt
            self.serialize_load_order();
        }
        // each frame we check the load order
        self.set_load_order();

        // Menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.menu_bar_view(ui, _frame);
        });

        // Left panel
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            self.load_order_view(ui);
        });

        // main conflicts view
        egui::CentralPanel::default().show(ctx, |ui| {
            self.conflicts_view(ui);
        });
    }
}

impl TemplateApp {
    fn load_order_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Load Order");
        ui.horizontal(|ui| {
            ui.label("Drag to reorder, higher overrides");
        });

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            egui_dnd::dnd(ui, "mod_list_dnd").show_vec(
                &mut self.load_order,
                |ui, f, handle, _state| {
                    ui.horizontal(|ui| {
                        handle.ui(ui, |ui| {
                            ui.label("::");
                        });
                        ui.label(f.clone());
                    });
                },
            );

            // ui.separator();
            // egui::Grid::new("mod_list").show(ui, |ui| {
            //     let mods = &self.load_order;
            //     for f in mods.iter() {
            //         ui.label(f);
            //         ui.end_row();
            //     }
            // });
        });
    }

    fn conflicts_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Conflicts");
        ui.separator();
        // -------------------
        ui.horizontal(|ui| {
            ui.label("Archives path");
            let mut path_str = self.game_path.to_str().unwrap();
            ui.text_edit_singleline(&mut path_str);
            if ui.button("...").clicked() {
                // open file
                if let Some(folder) = rfd::FileDialog::new().set_directory("/").pick_folder() {
                    self.game_path = folder;
                    // regenerate conflicts
                    self.last_load_order = None;
                }
            }
            // generate conflict map
            if ui.button("⟳ Re-check conflicts").clicked() && self.game_path.exists() {
                self.generate_conflict_map();
            }
        });
        ui.separator();
        // -------------------
        // Toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_no_conflicts, "Show not conflicting files");
            egui::ComboBox::from_label("Conflict style")
                .selected_text(format!("{:?}", &mut self.tooltips_visuals))
                .show_ui(ui, |ui| {
                    ui.selectable_value(
                        &mut self.tooltips_visuals,
                        ETooltipVisuals::Tooltip,
                        "Tooltip",
                    );
                    ui.selectable_value(
                        &mut self.tooltips_visuals,
                        ETooltipVisuals::Inline,
                        "Inline",
                    );
                    ui.selectable_value(
                        &mut self.tooltips_visuals,
                        ETooltipVisuals::Collapsing,
                        "Collapsing",
                    );
                });
        });
        // Filters
        ui.horizontal(|ui| {
            ui.label("Mod filter: ");
            ui.text_edit_singleline(&mut self.text_filter);
            if ui.button("x").clicked() {
                self.text_filter.clear();
            }
            ui.separator();
            ui.label("File filter: ");
            ui.text_edit_singleline(&mut self.file_filter);
            if ui.button("x").clicked() {
                self.file_filter.clear();
            }
        });
        ui.label(format!(
            "Found {} conflicts across {} archives",
            self.conflicts.len(),
            self.load_order.len()
        ));

        ui.separator();

        egui::ScrollArea::both()
            .auto_shrink([false; 2])
            .show(ui, |ui| {
                egui::Grid::new("mod_list").num_columns(1).show(ui, |ui| {
                    for archive_name in &self.load_order {
                        let archive_path = &self.game_path.join(archive_name);
                        let k = &fnv1a64_hash_path(archive_path);
                        if let Some(value) = self.archives.get(k) {
                            // skip if no conflicts
                            if value.loses.len() + value.wins.len() == 0 {
                                continue;
                            }

                            let filename = value
                                .path
                                .file_name()
                                .unwrap()
                                .to_ascii_lowercase()
                                .to_str()
                                .unwrap()
                                .to_owned();

                            // text filter
                            if !self.text_filter.is_empty()
                                && !filename
                                    .to_lowercase()
                                    .contains(&self.text_filter.to_lowercase())
                            {
                                continue;
                            }

                            let filename_ext = if !self.show_no_conflicts {
                                format!(
                                    "{} (w: {}, l: {})",
                                    filename,
                                    value.wins.len(),
                                    value.loses.len()
                                )
                            } else {
                                format!(
                                    "{} (w: {}, l: {}, u: {})",
                                    filename,
                                    value.wins.len(),
                                    value.loses.len(),
                                    value.get_no_conflicts().len()
                                )
                            };

                            ui.collapsing(filename_ext, |ui| {
                                let mut header_color = if value.wins.is_empty() {
                                    ui.visuals().text_color()
                                } else {
                                    Color32::GREEN
                                };
                                ui.collapsing(
                                    egui::RichText::new(format!("winning ({})", value.wins.len()))
                                        .color(header_color),
                                    |ui| {
                                        for h in &value.wins {
                                            // resolve hash
                                            let mut label_text = h.to_string();
                                            if let Some(file_name) = self.hashes.get(h) {
                                                label_text = file_name.to_owned();
                                            }

                                            // text filter
                                            if !self.file_filter.is_empty()
                                                && !label_text
                                                    .to_lowercase()
                                                    .contains(&self.file_filter.to_lowercase())
                                            {
                                                continue;
                                            }

                                            let color = Color32::GREEN;
                                            match self.tooltips_visuals {
                                                crate::ETooltipVisuals::Tooltip => {
                                                    show_tooltip(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        color,
                                                    );
                                                }
                                                crate::ETooltipVisuals::Inline => {
                                                    show_inline(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        color,
                                                    );
                                                }
                                                crate::ETooltipVisuals::Collapsing => {
                                                    show_dropdown_filelist(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        color,
                                                    );
                                                }
                                            }
                                        }
                                    },
                                );

                                header_color = if value.loses.is_empty() {
                                    ui.visuals().text_color()
                                } else {
                                    Color32::RED
                                };
                                ui.collapsing(
                                    egui::RichText::new(format!("losing ({})", value.loses.len()))
                                        .color(header_color),
                                    |ui| {
                                        for h in &value.loses {
                                            let mut label_text = h.to_string();
                                            if let Some(file_name) = self.hashes.get(h) {
                                                label_text = file_name.to_owned();
                                            }

                                            // text filter
                                            if !self.file_filter.is_empty()
                                                && !label_text
                                                    .to_lowercase()
                                                    .contains(&self.file_filter.to_lowercase())
                                            {
                                                continue;
                                            }

                                            let color = Color32::RED;
                                            match self.tooltips_visuals {
                                                crate::ETooltipVisuals::Tooltip => {
                                                    show_tooltip(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        color,
                                                    );
                                                }
                                                crate::ETooltipVisuals::Inline => {
                                                    show_inline(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        color,
                                                    );
                                                }
                                                crate::ETooltipVisuals::Collapsing => {
                                                    show_dropdown_filelist(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        color,
                                                    );
                                                }
                                            }
                                        }
                                    },
                                );

                                if self.show_no_conflicts {
                                    ui.collapsing(
                                        format!(
                                            "no conflicts ({})",
                                            value.get_no_conflicts().len()
                                        ),
                                        |ui| {
                                            for h in &value.get_no_conflicts() {
                                                let mut label_text = h.to_string();
                                                if let Some(file_name) = self.hashes.get(h) {
                                                    label_text = file_name.to_owned();
                                                }
                                                ui.label(label_text);
                                            }
                                        },
                                    );
                                }
                            });

                            ui.end_row();
                        }
                    }
                });
            });
    }

    fn menu_bar_view(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // The top panel is often a good place for a menu bar:
        egui::menu::bar(ui, |ui| {
            #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
            {
                ui.menu_button("File", |ui| {
                    if ui.button("Open log").clicked() {
                        let _ = open::that(format!("{}.log", crate::CARGO_PKG_NAME));

                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        _frame.close();
                    }
                });
                ui.add_space(16.0);
            }

            self.global_dark_light_mode_buttons(ui);

            egui::warn_if_debug_build(ui);
            ui.label(format!("v{}", crate::CARGO_PKG_VERSION));
        });
    }

    fn global_dark_light_mode_buttons(&mut self, ui: &mut egui::Ui) {
        let mut visuals = ui.ctx().style().visuals.clone();
        visuals.light_dark_radio_buttons(ui);
        ui.ctx().set_visuals(visuals);
        match ui.ctx().style().visuals.clone().dark_mode {
            true => self.theme = Some(ETheme::Dark),
            false => self.theme = Some(ETheme::Light),
        }
    }
}

fn show_inline(
    ui: &mut egui::Ui,
    label_text: String,
    h: &u64,
    k: &u64,
    conflicts: &HashMap<u64, Vec<u64>>,
    archive_map: &HashMap<u64, ArchiveViewModel>,
    color: Color32,
) {
    ui.horizontal(|ui| {
        ui.colored_label(color, label_text);
        // get archive names
        if let Some(archives) = conflicts.get(h) {
            for archive_hash in archives {
                if archive_hash == k {
                    continue;
                }

                let mut archive_name = archive_hash.to_string();
                if let Some(archive_vm) = archive_map.get(archive_hash) {
                    archive_name = archive_vm
                        .path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                }
                ui.label(archive_name);
            }
        }
    });
}

fn show_tooltip(
    ui: &mut egui::Ui,
    label_text: String,
    h: &u64,
    k: &u64,
    conflicts: &HashMap<u64, Vec<u64>>,
    archive_map: &HashMap<u64, ArchiveViewModel>,
    color: Color32,
) {
    let r = ui.colored_label(color, label_text);
    r.on_hover_ui(|ui| {
        // get archive names
        if let Some(archives) = conflicts.get(h) {
            for archive_hash in archives {
                if archive_hash == k {
                    continue;
                }

                let mut archive_name = archive_hash.to_string();
                if let Some(archive_vm) = archive_map.get(archive_hash) {
                    archive_name = archive_vm
                        .path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                }
                ui.label(archive_name);
            }
        }
    });
}

fn show_dropdown_filelist(
    ui: &mut egui::Ui,
    label_text: String,
    h: &u64,
    k: &u64,
    conflicts: &HashMap<u64, Vec<u64>>,
    archive_map: &HashMap<u64, ArchiveViewModel>,
    color: Color32,
) {
    ui.collapsing(egui::RichText::new(label_text).color(color), |ui| {
        // get archive names
        if let Some(archives) = conflicts.get(h) {
            for archive_hash in archives {
                if archive_hash == k {
                    continue;
                }

                let mut archive_name = archive_hash.to_string();
                if let Some(archive_vm) = archive_map.get(archive_hash) {
                    archive_name = archive_vm
                        .path
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                }
                ui.separator();
                ui.label(archive_name);
            }
        }
    });
}
