#![warn(clippy::all, rust_2018_idioms)]

pub mod scanner;

#[cfg(feature = "gui")]
mod app;

// Re-export so app.rs can still use `crate::ArchiveViewModel`
pub use scanner::ArchiveViewModel;

#[cfg(feature = "gui")]
const CARGO_PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
#[cfg(feature = "gui")]
const CARGO_PKG_NAME: &str = env!("CARGO_PKG_NAME");

#[cfg(feature = "gui")]
use log::error;
#[cfg(feature = "gui")]
use std::io::Write;
#[cfg(feature = "gui")]
use std::path::PathBuf;
#[cfg(feature = "gui")]
use std::collections::HashMap;

#[cfg(feature = "gui")]
#[derive(Default, serde::Deserialize, serde::Serialize, Debug, PartialEq)]
enum ETooltipVisuals {
    Tooltip,
    Inline,
    #[default]
    Collapsing,
}

/// We derive Deserialize/Serialize so we can persist app state on shutdown.
#[cfg(feature = "gui")]
#[derive(Default, serde::Deserialize, serde::Serialize)]
#[serde(default)]
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

#[cfg(feature = "gui")]
impl TemplateApp {
    /// Called once before the first frame.
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load previous app state (if any).
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }

        Default::default()
    }

    /// Returns the conflict map of this [`TemplateApp`]. Also sets archive and conflict maps
    fn generate_conflict_map(&mut self) {
        let old_archives = self.archives.clone();
        let result = scanner::scan_conflicts(
            &self.load_order,
            &self.game_path,
            Some(&old_archives),
        );
        self.archives = result.archives;
        self.conflicts = result.conflicts;
    }

    /// get modilist.txt path
    pub fn get_modlist_path(&self) -> PathBuf {
        self.game_path.join("modlist.txt")
    }

    /// Clear and regenerate load order
    pub fn reload_load_order(&mut self) {
        self.load_order = scanner::build_load_order(&self.game_path);
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
