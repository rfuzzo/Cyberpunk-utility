use crate::Archive;
extern crate egui;

use std::hash::Hasher;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use self::egui::Color32;

struct ArchiveViewModel {
    pub path: PathBuf,
    //pub hashes: HashMap<u64, String>,
    pub wins: Vec<u64>,
    pub looses: Vec<u64>,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    // Example stuff:
    game_path: PathBuf,

    #[serde(skip)] // This how you opt-out of serialization of a field
    archives: HashMap<u64, ArchiveViewModel>,
    #[serde(skip)] // This how you opt-out of serialization of a field
    load_order: Vec<u64>,
    #[serde(skip)] // This how you opt-out of serialization of a field
    conflicts: HashMap<u64, Vec<u64>>,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            game_path: PathBuf::from(""),
            archives: HashMap::default(),
            conflicts: HashMap::default(),
            load_order: vec![],
        }
    }
}

impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // This is also where you can customize the look and feel of egui using
        // `cc.egui_ctx.set_visuals` and `cc.egui_ctx.set_fonts`.

        // Load previous app state (if any).
        // Note that you must enable the `persistence` feature for this to work.
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    fn generate_conflict_map(&mut self, mut mods: Vec<PathBuf>) {
        self.load_order.clear();
        self.archives.clear();
        self.conflicts.clear();

        let mut conflict_map: HashMap<u64, Vec<u64>> = HashMap::default();
        let mut temp_load_order: Vec<u64> = vec![];

        // scan
        mods.reverse();
        for f in mods.iter() {
            if let Ok(archive) = Archive::from_file(f) {
                let mut hashes = archive.get_file_hashes();
                hashes.sort();
                let archive_hash = fnv1a64_hash_path(f);
                temp_load_order.push(archive_hash);

                let mut vm = ArchiveViewModel {
                    path: f.to_owned(),
                    //hashes: HashMap::default(),
                    wins: vec![],
                    looses: vec![],
                };

                for hash in hashes {
                    if let Some(archive_names) = conflict_map.get_mut(&hash) {
                        // found a conflict
                        // update vms
                        for archive in archive_names.iter() {
                            self.archives.get_mut(archive).unwrap().looses.push(hash);
                        }
                        archive_names.push(archive_hash);
                        vm.wins.push(hash);
                    } else {
                        // first occurance
                        conflict_map.insert(hash, vec![archive_hash]);
                    }
                }

                self.archives.insert(archive_hash, vm);
            }
        }

        // clean list
        let mut conflicts: HashMap<u64, Vec<u64>> = HashMap::default();
        for (hash, archives) in conflict_map.iter().filter(|p| p.1.len() > 1) {
            // insert
            conflicts.insert(*hash, archives.clone());
        }

        temp_load_order.reverse();
        self.conflicts = conflicts;
        self.load_order = temp_load_order;
    }
}

impl eframe::App for TemplateApp {
    /// Called by the frame work to save state before shutdown.
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    /// Called each time the UI needs repainting, which may be many times per second.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
        // For inspiration and more examples, go to https://emilk.github.io/egui

        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            // The top panel is often a good place for a menu bar:
            egui::menu::bar(ui, |ui| {
                #[cfg(not(target_arch = "wasm32"))] // no File->Quit on web pages!
                {
                    ui.menu_button("File", |ui| {
                        if ui.button("Quit").clicked() {
                            _frame.close();
                        }
                    });
                    ui.add_space(16.0);
                }

                egui::widgets::global_dark_light_mode_buttons(ui);
            });
        });

        let mods = get_files(&self.game_path, "archive");

        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            ui.heading("Installed Mods");
            ui.separator();

            egui::ScrollArea::vertical().show(ui, |ui| {
                egui::Grid::new("mod_list").show(ui, |ui| {
                    for f in mods.iter() {
                        ui.label(
                            f.file_name()
                                .unwrap()
                                .to_ascii_lowercase()
                                .to_str()
                                .unwrap(),
                        );
                        ui.end_row();
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Conflicts");
            ui.separator();
            // -------------------
            ui.horizontal(|ui| {
                ui.label("Game path");
                let mut path_str = self.game_path.to_str().unwrap();
                ui.text_edit_singleline(&mut path_str);
                if ui.button("...").clicked() {
                    // open file
                    if let Some(folder) = rfd::FileDialog::new().set_directory("/").pick_folder() {
                        self.game_path = folder;
                    }
                }
            });

            // generate conflict map
            if ui.button("check").clicked() && self.game_path.exists() {
                self.generate_conflict_map(mods);
            }

            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false; 2])
                .show(ui, |ui| {
                    egui::Grid::new("mod_list").num_columns(1).show(ui, |ui| {
                        for k in &self.load_order {
                            if let Some(value) = self.archives.get(k) {
                                let filename = value
                                    .path
                                    .file_name()
                                    .unwrap()
                                    .to_ascii_lowercase()
                                    .to_str()
                                    .unwrap()
                                    .to_owned();

                                ui.collapsing(filename, |ui| {
                                    ui.collapsing(
                                        format!("winning ({})", value.wins.len()),
                                        |ui| {
                                            for h in &value.wins {
                                                ui.colored_label(Color32::GREEN, h.to_string());
                                            }
                                        },
                                    );
                                    ui.collapsing(
                                        format!("loosing ({})", value.looses.len()),
                                        |ui| {
                                            for h in &value.looses {
                                                ui.colored_label(Color32::RED, h.to_string());
                                            }
                                        },
                                    );
                                });

                                ui.end_row();
                            }
                        }
                    });
                });
        });
    }
}

/// Get top-level files of a folder with given extension
fn get_files(folder_path: &Path, extension: &str) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if !folder_path.exists() {
        return files;
    }

    if let Ok(entries) = fs::read_dir(folder_path) {
        for entry in entries.flatten() {
            if let Ok(file_type) = entry.file_type() {
                if file_type.is_file() {
                    if let Some(ext) = entry.path().extension() {
                        if ext == extension {
                            files.push(entry.path());
                        }
                    }
                }
            }
        }
    }

    files
}

/// Calculate FNV1a64 hash of a PathBuf
fn fnv1a64_hash_path(path: &Path) -> u64 {
    let path_string = path.to_string_lossy();
    let mut hasher = fnv::FnvHasher::default();
    hasher.write(path_string.as_bytes());
    hasher.finish()
}
