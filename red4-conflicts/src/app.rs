use std::{collections::HashMap, env};

use egui::{Color32, Popup, UiKind};
use red4lib::{fnv1a64_hash_path, get_red4_hashes};

use crate::{ArchiveViewModel, ETooltipVisuals, TemplateApp};

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

        // each frame we check the load order
       
        // auto-generate hashes on first load and load order change
        if let Some(last_load_order) = &self.last_load_order {
            if &self.load_order != last_load_order {
                self.generate_conflict_map();
                self.last_load_order = Some(self.load_order.clone());
                self.serialize_load_order();
            }
        } else {
            // first load
            self.reload_load_order();
            self.generate_conflict_map();
            self.last_load_order = Some(self.load_order.clone());
            self.serialize_load_order();
        }

        // Menu bar
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.menu_bar_view(ui, ctx);
        });

        // Left panel
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            self.load_order_view(ui, ctx);
        });

        // main conflicts view
        egui::CentralPanel::default().show(ctx, |ui| {
            self.conflicts_view(ui);
        });

       
    }
}

impl TemplateApp {
    /// Side panel with a mod list in correct order
    fn load_order_view(&mut self,  ui: &mut egui::Ui, ctx: &egui::Context) {
        ui.heading("Load Order");
        ui.label("Drag to reorder, higher overrides");
        ui.horizontal(|ui| {
            if ui.checkbox(&mut self.enable_modlist, "Enable load order re-ordering").clicked() {
                // if toggled on, display a warning that the user understands the risk
                if self.enable_modlist {
                    match rfd::MessageDialog::new()
                        .set_title("Enable load order management")
                        .set_description("Enabling load order management will create or overwrite a file named \"modlist.txt\" in your /archive/pc/mod folder.\n\n\
                        This file will determine the load order of your mods and can seriously mess up your game if not used correctly.\n\n\
                        Are you sure you understand and want to enable this feature?")
                        .set_buttons(rfd::MessageButtons::OkCancel)
                        .set_level(rfd::MessageLevel::Warning)
                        .show() {
                            rfd::MessageDialogResult::Yes => {},
                            rfd::MessageDialogResult::Ok => {},
                            rfd::MessageDialogResult::No => {
                                self.enable_modlist = false;
                                self.reset_loadorder();
                                return;
                            },
                            rfd::MessageDialogResult::Cancel => {   
                                self.enable_modlist = false;
                                self.reset_loadorder();
                                return;
                            },
                            rfd::MessageDialogResult::Custom(_) => {},
                        }
                }
                else {
                    self.reset_loadorder();
                }
               
               
            };
            
            
            let response = ui.button("？");
            let popup_id = ui.make_persistent_id("my_unique_id");
            if response.clicked() {
                Popup::toggle_id(ctx, popup_id);
            }
            // open info
            response.on_hover_ui_at_pointer(|ui| {
                ui.set_min_width(400.0); // if you want to control the size
                ui.heading("Cyberpunk 2077 load order");
                ui.label("Archives in Cyberpunk are loaded binary-alphabetically.");
                ui.label("This means that a mod called \"modaa\" loads before \"modbb\", but \"modA\" loads before \"modaa\" and \"modbb\".");
                ui.label("Special characters also load according to binary sorting: \"!\" and \"#\" before \"A\", but \"_\" after \"Z\". Check the ASCII character set for more info:");
                ui.hyperlink("https://en.wikipedia.org/wiki/ASCII#Character_set/");
                ui.label("All REDmod archives are strictly loaded after archives in the /archive/pc/mod folder.");
                
                ui.add_space(16.0);
                ui.heading("Modlist.txt");
                ui.label("The game provides a way to adjust archive load order without renaming the files: Archives in \"modlist.txt\" in your /archive/pc/mod folder are loaded according to this list.");
                ui.label("Reordering mods in this app will generate this file.");
            });
        });

        // reset load order button
        if self.enable_modlist &&  ui.button("⟳  Reset load order").clicked() {
            self.reset_loadorder();
        }

        ui.separator();

        egui::ScrollArea::vertical().show(ui, |ui| {
            if self.enable_modlist {
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
            } else {
                egui::Grid::new("mod_list").show(ui, |ui| {
                    let mods = &self.load_order;
                    for f in mods.iter() {
                        ui.label(f);
                        ui.end_row();
                    }
                });
            }
        });
    }

    fn reset_loadorder(&mut self) {
        // delete modlist.txt and reload
        let modlist_path = self.get_modlist_path();
        if modlist_path.exists() {
            if let Err(e) = std::fs::remove_file(&modlist_path) {
                log::error!("Failed to delete modlist.txt: {}", e);
            }
        }
        self.reload_load_order();
        self.generate_conflict_map();
        self.last_load_order = Some(self.load_order.clone());
        if self.enable_modlist {
            self.serialize_load_order();
        }
    }
    
    /// Main conflict grid
    fn conflicts_view(&mut self, ui: &mut egui::Ui) {
        ui.heading("Conflicts");
        ui.separator();
        // -------------------
        ui.horizontal(|ui| {
            ui.label("Archives path");
            if let Some(mut path_str) = self.game_path.to_str() {
                ui.text_edit_singleline(&mut path_str);
            }
            if ui.button("...").clicked() {
                // open file
                if let Some(folder) = rfd::FileDialog::new().set_directory("/").pick_folder() {
                    self.game_path = folder;
                    // regenerate conflicts
                    self.last_load_order = None;
                }
            }
            // generate conflict map
            if ui.button("⟳  Re-check conflicts").clicked() && self.game_path.exists() {
                self.reload_load_order();
                self.generate_conflict_map();
                self.last_load_order = Some(self.load_order.clone());
            }
            if ui.button("🗁 Open in Explorer").clicked() && self.game_path.exists() {
                let _ = open::that(self.game_path.clone());
            }
        });
        ui.separator();
        // -------------------
        // Toolbar
        ui.horizontal(|ui| {
            ui.checkbox(&mut self.show_no_conflicts, "Show not conflicting files");
            ui.label("Conflict style");
            egui::ComboBox::from_id_salt("tooltips_visuals")
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
                egui::Grid::new("mod_list").num_columns(2).show(ui, |ui| {
                    for archive_name in &self.load_order {
                        let archive_path = &self.game_path.join(archive_name);
                        let k = &fnv1a64_hash_path(archive_path);
                        if let Some(mod_vm) = self.archives.get(k) {
                            // skip if no conflicts
                            if mod_vm.loses.len() + mod_vm.wins.len() == 0 {
                                continue;
                            }

                            // text filter
                            if !self.text_filter.is_empty()
                                && !mod_vm.file_name
                                    .to_lowercase()
                                    .contains(&self.text_filter.to_lowercase())
                            {
                                continue;
                            }

                            let filename_ext = if !self.show_no_conflicts {
                                format!(
                                    "{} (w: {}, l: {})",
                                    mod_vm.file_name,
                                    mod_vm.wins.len(),
                                    mod_vm.loses.len()
                                )
                            } else {
                                format!(
                                    "{} (w: {}, l: {}, u: {})",
                                    mod_vm.file_name,
                                    mod_vm.wins.len(),
                                    mod_vm.loses.len(),
                                    mod_vm.get_no_conflicts().len()
                                )
                            };

                            // column 1
                            ui.collapsing(filename_ext, |ui| {
                                let mut header_color = if mod_vm.wins.is_empty() {
                                    ui.visuals().text_color()
                                } else {
                                    Color32::GREEN
                                };
                                ui.collapsing(
                                    egui::RichText::new(format!("winning ({})", mod_vm.wins.len()))
                                        .color(header_color),
                                    |ui| {
                                        for h in &mod_vm.wins {
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

                                            match self.tooltips_visuals {
                                                crate::ETooltipVisuals::Tooltip => {
                                                    show_tooltip(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        true,
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
                                                        true
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
                                                        true,
                                                    );
                                                }
                                            }
                                        }
                                    },
                                );

                                header_color = if mod_vm.loses.is_empty() {
                                    ui.visuals().text_color()
                                } else {
                                    Color32::RED
                                };
                                ui.collapsing(
                                    egui::RichText::new(format!("losing ({})", mod_vm.loses.len()))
                                        .color(header_color),
                                    |ui| {
                                        for h in &mod_vm.loses {
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

                                            match self.tooltips_visuals {
                                                crate::ETooltipVisuals::Tooltip => {
                                                    show_tooltip(
                                                        ui,
                                                        label_text,
                                                        h,
                                                        k,
                                                        &self.conflicts,
                                                        &self.archives,
                                                        false,
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
                                                        false
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
                                                        false,
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
                                            mod_vm.get_no_conflicts().len()
                                        ),
                                        |ui| {
                                            for h in &mod_vm.get_no_conflicts() {
                                                let mut label_text = h.to_string();
                                                if let Some(file_name) = self.hashes.get(h) {
                                                    label_text = file_name.to_owned();
                                                }
                                                ui.add(egui::Label::new(label_text).wrap_mode(egui::TextWrapMode::Truncate));
                                            }
                                        },
                                    );
                                }
                            });

                            // column 2
                            ui.horizontal_top(|ui|
                            {
                                // if all files of a mod are losing then its obsolete
                                if (mod_vm.files.len() == mod_vm.loses.len()) && mod_vm.wins.is_empty() {
                                    ui.colored_label( Color32::GRAY, "⏺");
                                }
                                else {
                                     // if some files are winning add green dot
                                    if !mod_vm.wins.is_empty() {
                                        ui.colored_label( Color32::GREEN, "⏺");
                                    }
                                    // if some files are losing add red dot
                                    if !mod_vm.loses.is_empty() {
                                        ui.colored_label( Color32::RED, "⏺");
                                    }
                                }
                            });
                            
                            ui.end_row();
                                                        
                        }
                    }
                });
            });
    }

    /// The menu bar
    fn menu_bar_view(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        // The top panel is often a good place for a menu bar:
        egui::MenuBar::new().ui(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Open modlist.txt").clicked() {
                    let _ = open::that(self.get_modlist_path());
                    ui.close_kind(UiKind::Menu);
                }
                ui.separator();
                if ui.button("Quit").clicked() {
                  ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
            });
            ui.menu_button("About", |ui| {
                ui.hyperlink("https://github.com/rfuzzo/Cyberpunk-utility/");
                ui.separator();
                if ui.button("Open log").clicked() {
                    let _ = open::that(format!("{}.log", crate::CARGO_PKG_NAME));

                    ui.close_kind(UiKind::Menu);
                }
            });
            ui.add_space(16.0);

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::global_theme_preference_buttons(ui);
                
                ui.add_space(16.0);
                
                ui.separator();
                egui::warn_if_debug_build(ui);
                ui.label(format!("v{}", crate::CARGO_PKG_VERSION));
            });
        });
    }

   
}

fn get_archive_hashes_for_ui(winning: bool, archives: &[u64], key: &u64) -> Vec<u64> {
    let mut stop_skip = false;
    let mut final_names = vec![];

    let archives = if winning {
        archives.iter().rev().collect::<Vec<_>>()
    } else {
        archives.iter().collect::<Vec<_>>()
    };

    for archive_hash in archives {
        if archive_hash == key {
            stop_skip = true;
            continue;
        }

        if stop_skip {
            final_names.push(*archive_hash);
        }
    }

    if !winning {
        final_names.reverse();
    }
    final_names
}

fn show_inline(
    ui: &mut egui::Ui,
    label_text: String,
    h: &u64,
    key: &u64,
    conflicts: &HashMap<u64, Vec<u64>>,
    archive_map: &HashMap<u64, ArchiveViewModel>,
    winning: bool
) {
    ui.horizontal(|ui| {
        let color = if winning {
            Color32::GREEN
        } else {
            Color32::RED
        };
        ui.colored_label(color, label_text);
        // get archive names
        if let Some(archives) = conflicts.get(h) {
            for archive_hash in get_archive_hashes_for_ui(winning, archives, key) {
                let archive_name = if let Some(archive_vm) = archive_map.get(&archive_hash) {
                    archive_vm.file_name.to_owned()
                } else {
                    archive_hash.to_string()
                };
                ui.label(archive_name);
            }

        }
    });
}



fn show_tooltip(
    ui: &mut egui::Ui,
    label_text: String,
    h: &u64,
    key: &u64,
    conflicts: &HashMap<u64, Vec<u64>>,
    archive_map: &HashMap<u64, ArchiveViewModel>,
    winning: bool
) {
    let color = if winning {
        Color32::GREEN
    } else {
        Color32::RED
    };
    let r = ui.colored_label(color, label_text);
    r.on_hover_ui(|ui| {
        // get archive names
        if let Some(archives) = conflicts.get(h) {
            for archive_hash in get_archive_hashes_for_ui(winning, archives, key) {
               
                let archive_name = if let Some(archive_vm) = archive_map.get(&archive_hash) {
                    archive_vm.file_name.to_owned()
                } else {
                    archive_hash.to_string()
                };
                ui.label(archive_name);
            }
        }
    });
}

fn show_dropdown_filelist(
    ui: &mut egui::Ui,
    label_text: String,
    h: &u64,
    key: &u64,
    conflicts: &HashMap<u64, Vec<u64>>,
    archive_map: &HashMap<u64, ArchiveViewModel>,
    winning: bool
) {
    let color = if winning {
        Color32::GREEN
    } else {
        Color32::RED
    };
    ui.collapsing(egui::RichText::new(label_text).color(color), |ui| {
        // get archive names
        if let Some(archives) = conflicts.get(h) {
            for archive_hash in archives {
                if archive_hash == key {
                    continue;
                }

                let archive_name = if let Some(archive_vm) = archive_map.get(archive_hash) {
                    archive_vm.file_name.to_owned()
                } else {
                    archive_hash.to_string()
                };
                //ui.separator();
                ui.label(archive_name);
            }
        }
    });
}
