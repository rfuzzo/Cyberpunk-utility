#![warn(clippy::all, rust_2018_idioms)]

use std::{collections::HashMap, path::PathBuf};

use red4lib::{fnv1a64_hash_path, fnv1a64_hash_string, Archive};

mod app;

struct ArchiveViewModel {
    pub path: PathBuf,
    /// winning file hashes
    pub wins: Vec<u64>,
    /// losing file hashes
    pub loses: Vec<u64>,
    /// all file hashes
    pub hashes: Vec<u64>,
}

impl ArchiveViewModel {
    pub fn get_no_conflicts(&self) -> Vec<u64> {
        let result: Vec<u64> = self
            .hashes
            .iter()
            .filter(|&x| !self.wins.contains(x))
            .filter(|&x| !self.loses.contains(x))
            .cloned()
            .collect();
        result
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum ETooltipVisuals {
    Tooltip,
    Inline,
    Collapsing,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    #[serde(skip)] // This how you opt-out of serialization of a field
    game_path: PathBuf,
    #[serde(skip)]
    hashes: HashMap<u64, String>,
    #[serde(skip)]
    archives: HashMap<u64, ArchiveViewModel>,
    /// archive hash load order
    #[serde(skip)]
    load_order: Vec<u64>,
    /// map of file hashes to archive hashes
    #[serde(skip)]
    conflicts: HashMap<u64, Vec<u64>>,

    #[serde(skip)]
    text_filter: String,

    show_no_conflicts: bool,
    tooltips_visuals: ETooltipVisuals,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            hashes: HashMap::default(),
            game_path: PathBuf::from(""),
            archives: HashMap::default(),
            conflicts: HashMap::default(),
            load_order: vec![],
            text_filter: "".into(),
            show_no_conflicts: false,
            tooltips_visuals: ETooltipVisuals::Collapsing,
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
            log::info!("parsing {}", f.display());

            if let Ok(archive) = Archive::from_file(f) {
                // add custom filenames
                for f in archive.file_names.iter() {
                    let key = fnv1a64_hash_string(f);
                    self.hashes.insert(key, f.to_string());
                }

                // conflicts
                let mut hashes = archive.get_file_hashes();
                hashes.sort();
                let archive_hash = fnv1a64_hash_path(f);
                temp_load_order.push(archive_hash);

                let mut vm = ArchiveViewModel {
                    path: f.to_owned(),
                    hashes: hashes.clone(),
                    wins: vec![],
                    loses: vec![],
                };

                for hash in hashes {
                    if let Some(archive_names) = conflict_map.get_mut(&hash) {
                        // found a conflict
                        // update vms
                        // add this file to all previous archive's losing files
                        for archive in archive_names.iter() {
                            if !self.archives.get(archive).unwrap().loses.contains(&hash) {
                                self.archives.get_mut(archive).unwrap().loses.push(hash);
                            }
                        }
                        // add the current archive to the list of conflicting archives last
                        if !archive_names.contains(&archive_hash) {
                            archive_names.push(archive_hash);
                        }
                        // add this file to this mods winning files
                        if !vm.wins.contains(&hash) {
                            vm.wins.push(hash);
                        }
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
