#![warn(clippy::all, rust_2018_idioms)]

use log::error;
use red4lib::fnv1a64_hash_path;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

mod app;

const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[derive(Clone)]
struct ArchiveViewModel {
    pub file_name: String,
    /// winning file hashes
    pub wins: Vec<u64>,
    /// losing file hashes
    pub loses: Vec<u64>,
    /// all file hashes
    pub files: Vec<u64>,
}

impl ArchiveViewModel {
    pub fn get_no_conflicts(&self) -> Vec<u64> {
        let result: Vec<u64> = self
            .files
            .iter()
            .filter(|&x| !self.wins.contains(x))
            .filter(|&x| !self.loses.contains(x))
            .cloned()
            .collect();
        result
    }
}

#[derive(Default, serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum ETooltipVisuals {
    Tooltip,
    Inline,
    #[default]
    Collapsing,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct TemplateApp {
    game_path: PathBuf,
    // UI
    show_no_conflicts: bool,
    /// the way conflicts are disaplyed in the conflicts view
    tooltips_visuals: ETooltipVisuals,

    /// enables load order management via modlist.txt
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

    // UI filters
    #[serde(skip)]
    text_filter: String,
    #[serde(skip)]
    file_filter: String,
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
        let old_archives = self.archives.clone();
        self.archives.clear();
        self.conflicts.clear();

        let mut conflict_map: HashMap<u64, Vec<u64>> = HashMap::default();

        // scan
        let mut mods = self.load_order.clone();
        mods.reverse();

        for archive_name in mods.iter() {
            let archive_file_path = &self.game_path.join(archive_name);
            let archive_hash = fnv1a64_hash_path(archive_file_path);
            log::info!("parsing {}", archive_file_path.display());

            // read or get the archive
            let mut archive_or_none: Option<ArchiveViewModel> = None;
            if let Some(archive) = old_archives.get(&archive_hash) {
                // no need to read the file again
                let mut empty_vm = archive.clone();
                // but clean it since we're calculating conflicts anew
                empty_vm.wins.clear();
                empty_vm.loses.clear();

                archive_or_none = Some(empty_vm);
            } else if let Ok(archive) = red4lib::archive::open_read(archive_file_path) {
                if let Some(archive_file_name) =
                    archive_file_path.file_name().and_then(|f| f.to_str())
                {
                    // conflicts
                    let mut hashes = archive
                        .get_entries()
                        .clone()
                        .into_keys()
                        .collect::<Vec<_>>();
                    hashes.sort();

                    let vm = ArchiveViewModel {
                        file_name: archive_file_name.to_owned(),
                        files: hashes.clone(),
                        wins: vec![],
                        loses: vec![],
                    };

                    archive_or_none = Some(vm);
                }
            }

            if let Some(mut archive_vm) = archive_or_none {
                for hash in &archive_vm.files {
                    if let Some(archive_names) = conflict_map.get_mut(hash) {
                        // found a conflict
                        // update vms
                        // add this file to all previous archive's losing files
                        for archive in archive_names.iter() {
                            let archive_vm = self.archives.get_mut(archive).unwrap();
                            if !archive_vm.loses.contains(hash) {
                                archive_vm.loses.push(*hash);
                            }
                            // remove from wins if previously marked as winning
                            if let Some(pos) = archive_vm.wins.iter().position(|w| w == hash) {
                                archive_vm.wins.remove(pos);
                            }
                        }
                        // add the current archive to the list of conflicting archives last
                        if !archive_names.contains(&archive_hash) {
                            archive_names.push(archive_hash);
                        }
                        // add this file to this mods winning files
                        if !archive_vm.wins.contains(hash) {
                            archive_vm.wins.push(*hash);
                        }
                    } else {
                        // first occurance
                        conflict_map.insert(*hash, vec![archive_hash]);
                    }
                }

                self.archives.insert(archive_hash, archive_vm);
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

    /// get modilist.txt path
    pub fn get_modlist_path(&self) -> PathBuf {
        self.game_path.join("modlist.txt")
    }

    /// Clear and regenerate load order
    pub fn reload_load_order(&mut self) {
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

        if let Ok(lines) = read_file_to_vec(&self.get_modlist_path()) {
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

        self.load_order = pathbuf_to_string_vec(final_order);
    }

    fn serialize_load_order(&self) {
        if !self.enable_modlist {
            return;
        }

        if let Ok(mut file) = std::fs::File::create(self.get_modlist_path()) {
            for line in &self.load_order {
                let new_line = format!("{}\r\n", line);
                match file.write_all(new_line.as_bytes()) {
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
