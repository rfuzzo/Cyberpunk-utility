#![warn(clippy::all, rust_2018_idioms)]

use log::error;
use red4lib::archive::Archive;
use red4lib::{fnv1a64_hash_path, fnv1a64_hash_string, get_files};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::{collections::HashMap, path::PathBuf};

mod app;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

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

#[derive(serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum ETheme {
    Dark,
    Light,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    game_path: PathBuf,
    // UI
    show_no_conflicts: bool,
    tooltips_visuals: ETooltipVisuals,
    theme: Option<ETheme>,
    enable_modlist: bool,

    /// hash DB
    #[serde(skip)]
    hashes: HashMap<u64, String>,
    /// archive name lookup
    #[serde(skip)]
    archives: HashMap<u64, ArchiveViewModel>,
    /// map of file hashes to archive hashes
    #[serde(skip)]
    conflicts: HashMap<u64, Vec<u64>>,

    /// archive hash load order
    #[serde(skip)]
    load_order: Vec<String>,
    #[serde(skip)]
    last_load_order: Option<Vec<String>>,

    // UI
    #[serde(skip)]
    text_filter: String,
    #[serde(skip)]
    file_filter: String,
}

impl Default for TemplateApp {
    fn default() -> Self {
        Self {
            theme: None,
            hashes: HashMap::default(),
            game_path: PathBuf::from(""),
            archives: HashMap::default(),
            conflicts: HashMap::default(),
            load_order: vec![],
            last_load_order: None,
            text_filter: "".into(),
            file_filter: "".into(),
            show_no_conflicts: false,
            enable_modlist: false,
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

    /// Returns the conflict map of this [`TemplateApp`]. Also sets archive and conflict maps
    fn generate_conflict_map(&mut self) {
        self.archives.clear();
        self.conflicts.clear();

        let mut conflict_map: HashMap<u64, Vec<u64>> = HashMap::default();
        //let mut temp_load_order: Vec<PathBuf> = vec![];

        // scan
        let mut mods = self.load_order.clone();
        mods.reverse();
        for archive_name in mods.iter() {
            let file_path = &self.game_path.join(archive_name);
            log::info!("parsing {}", file_path.display());

            if let Ok(archive) = Archive::from_file(file_path) {
                // add custom filenames
                for f in archive.file_names.values() {
                    let key = fnv1a64_hash_string(f);
                    self.hashes.insert(key, f.to_string());
                }

                // conflicts
                let mut hashes = archive.get_file_hashes();
                hashes.sort();
                let archive_hash = fnv1a64_hash_path(file_path);
                //temp_load_order.push(file_path.to_owned());

                let mut vm = ArchiveViewModel {
                    path: file_path.to_owned(),
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

        //temp_load_order.reverse();
        self.conflicts = conflicts;
    }

    fn read_file_to_vec(file_path: &PathBuf) -> io::Result<Vec<String>> {
        let file = File::open(file_path)?;
        let reader = BufReader::new(file);

        let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();

        Ok(lines)
    }

    fn pathbuf_to_string_vec(paths: Vec<PathBuf>) -> Vec<String> {
        paths
            .into_iter()
            .filter_map(|path| {
                path.file_name()
                    .map(|filename| filename.to_string_lossy().into_owned())
            })
            .collect()
    }

    /// Clear and regenerate load order
    pub fn set_load_order(&mut self) {
        self.load_order.clear();

        let mut mods: Vec<PathBuf> = get_files(&self.game_path, "archive");
        // load order
        mods.sort_by(|a, b| {
            a.to_string_lossy()
                .as_bytes()
                .cmp(b.to_string_lossy().as_bytes())
        });
        // load according to modlist.txt
        let mut final_order: Vec<PathBuf> = vec![];
        let modlist_name = "modlist.txt";
        if let Ok(lines) = Self::read_file_to_vec(&self.game_path.join(modlist_name)) {
            for name in lines {
                let file_name = self.game_path.join(name);
                if mods.contains(&file_name) {
                    final_order.push(file_name.to_owned());
                }
            }
            // add remaining mods last
            for m in mods {
                if !final_order.contains(&m) {
                    final_order.push(m);
                }
            }
        } else {
            final_order = mods;
        }
        // TODO Redmods

        self.load_order = Self::pathbuf_to_string_vec(final_order);
    }

    fn serialize_load_order(&self) {
        if !self.enable_modlist {
            return;
        }

        let modlist_name = "modlist.txt";
        if let Ok(mut file) = std::fs::File::create(self.game_path.join(modlist_name)) {
            for line in &self.load_order {
                match writeln!(file, "{}", line) {
                    Ok(_) => {}
                    Err(err) => {
                        error!("failed to write line {}", err);
                    }
                }
            }
        } else {
            error!("failed to write load order");
        }
    }
}
