//! Shared conflict detection logic for both GUI and CLI.
//! No GUI dependencies in this module.

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader};
use std::path::{Path, PathBuf};

use red4lib::fnv1a64_hash_path;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Core data types
// ---------------------------------------------------------------------------

#[derive(Clone)]
pub struct ArchiveViewModel {
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
        self.files
            .iter()
            .filter(|&x| !self.wins.contains(x))
            .filter(|&x| !self.loses.contains(x))
            .cloned()
            .collect()
    }
}

pub struct ConflictScanResult {
    pub archives: HashMap<u64, ArchiveViewModel>,
    /// file_hash → vec of archive hashes (in load-priority order, last wins)
    pub conflicts: HashMap<u64, Vec<u64>>,
}

// ---------------------------------------------------------------------------
// JSON report types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ConflictReport {
    /// Scanned archive folder
    pub archive_path: String,
    /// Full load order list (first = lowest priority, last = highest).
    /// Only included when explicitly requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_order: Option<Vec<String>>,
    /// Total number of .archive files found
    pub total_archives: usize,
    /// Total number of files that appear in more than one archive
    pub total_conflicting_files: usize,
    /// Per-archive breakdown
    pub archives: Vec<ArchiveReport>,
    /// Per-file conflict details (only files present in 2+ archives)
    pub file_conflicts: Vec<FileConflict>,
}

#[derive(Serialize)]
pub struct ArchiveReport {
    /// Archive file name
    pub name: String,
    /// 0-based position in load order (higher index = higher priority = wins)
    pub load_order_index: usize,
    /// Total files inside the archive
    pub total_files: usize,
    /// Number of files this archive wins (overrides others)
    pub winning_count: usize,
    /// Number of files this archive loses (overridden by a later archive)
    pub losing_count: usize,
    /// "no_conflicts" | "has_conflicts" | "fully_overridden"
    pub status: String,
    /// Files this archive wins — with names of archives it overrides
    pub winning_files: Vec<ConflictFileEntry>,
    /// Files this archive loses — with name of the archive that overrides it
    pub losing_files: Vec<ConflictFileEntry>,
    /// Files with no conflicts (only present when show_no_conflicts is enabled)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub no_conflict_files: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct ConflictFileEntry {
    /// Human-readable file path, or raw hash if unknown
    pub file: String,
    /// Archives involved in this specific conflict (excluding current archive)
    pub conflicting_archives: Vec<String>,
}

#[derive(Serialize)]
pub struct FileConflict {
    /// Human-readable file path, or raw hash if unknown
    pub file: String,
    /// All archives containing this file, ordered by load priority (last wins)
    pub archives: Vec<String>,
    /// The archive that currently wins (highest in load order)
    pub winner: String,
}

// ---------------------------------------------------------------------------
// Summary report types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct SummaryReport {
    pub archive_path: String,
    pub total_archives: usize,
    pub total_conflicting_files: usize,
    /// Number of mods that are completely overridden (every file lost, nothing wins)
    pub fully_overridden_count: usize,
    /// One entry per archive that has conflicts
    pub archives: Vec<ArchiveSummary>,
}

#[derive(Serialize)]
pub struct ArchiveSummary {
    pub name: String,
    pub load_order_index: usize,
    pub total_files: usize,
    pub winning_count: usize,
    pub losing_count: usize,
    /// true when every file in this archive is overridden and nothing wins —
    /// the mod has zero effect in-game, which may indicate a load order problem
    pub fully_overridden: bool,
}

// ---------------------------------------------------------------------------
// Core functions
// ---------------------------------------------------------------------------

/// Build load order from the archive folder.
/// Reads `modlist.txt` from `game_path` if present; remaining archives are
/// appended in binary-alphabetical order (matching game behaviour).
pub fn build_load_order(game_path: &Path) -> Vec<String> {
    let mut mods = get_files(game_path, "archive");
    mods.sort_by(|a, b| {
        a.to_string_lossy()
            .as_bytes()
            .cmp(b.to_string_lossy().as_bytes())
    });

    let mut final_order: Vec<PathBuf> = Vec::new();

    let modlist_path = game_path.join("modlist.txt");
    if let Ok(lines) = read_file_to_vec(&modlist_path) {
        for name in lines {
            let file_name = game_path.join(&name);
            if mods.contains(&file_name) {
                final_order.push(file_name);
            }
        }
        // add remaining mods last
        for m in &mods {
            if !final_order.contains(m) {
                final_order.push(m.clone());
            }
        }
    } else {
        final_order = mods;
    }

    pathbuf_to_string_vec(final_order)
}

/// Scan archives and build the conflict map.
/// `load_order` is lowest-priority first, highest-priority last.
///
/// When `old_archives` is `Some`, cached archive data is reused (with
/// wins/loses cleared) instead of re-reading archive files from disk.
/// The CLI passes `None` to always read fresh.
pub fn scan_conflicts(
    load_order: &[String],
    game_path: &Path,
    old_archives: Option<&HashMap<u64, ArchiveViewModel>>,
) -> ConflictScanResult {
    let mut archives: HashMap<u64, ArchiveViewModel> = HashMap::new();
    let mut conflict_map: HashMap<u64, Vec<u64>> = HashMap::new();

    // Iterate in reverse so the *last* archive processed is the highest-priority
    let mut mods_reversed = load_order.to_vec();
    mods_reversed.reverse();

    for archive_name in &mods_reversed {
        let archive_file_path = game_path.join(archive_name);
        let archive_hash = fnv1a64_hash_path(&archive_file_path);
        log::info!("parsing {}", archive_file_path.display());

        // Try cache first, then read from disk
        let archive_vm = if let Some(old) = old_archives {
            if let Some(cached) = old.get(&archive_hash) {
                let mut vm = cached.clone();
                vm.wins.clear();
                vm.loses.clear();
                Some(vm)
            } else {
                read_archive_vm(&archive_file_path)
            }
        } else {
            read_archive_vm(&archive_file_path)
        };

        let Some(mut vm) = archive_vm else {
            log::warn!("Skipping {}", archive_name);
            continue;
        };

        for hash in &vm.files {
            if let Some(existing) = conflict_map.get_mut(hash) {
                // Mark all previous archives as losing this file
                for prev_hash in existing.iter() {
                    if let Some(prev) = archives.get_mut(prev_hash) {
                        if !prev.loses.contains(hash) {
                            prev.loses.push(*hash);
                        }
                    }
                }
                // Current archive wins this file
                if !existing.contains(&archive_hash) {
                    existing.push(archive_hash);
                }
                if !vm.wins.contains(hash) {
                    vm.wins.push(*hash);
                }
            } else {
                conflict_map.insert(*hash, vec![archive_hash]);
            }
        }

        archives.insert(archive_hash, vm);
    }

    // Keep only entries with actual conflicts (2+ archives)
    let conflicts: HashMap<u64, Vec<u64>> = conflict_map
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .collect();

    ConflictScanResult {
        archives,
        conflicts,
    }
}

/// Build a JSON-serializable report from scan results.
///
/// When `show_no_conflicts` is true, each archive includes a list of files
/// that have no conflicts. Archives with zero wins and zero losses are always
/// excluded (matching GUI behaviour).
///
/// When `include_load_order` is true, the full load order list is included
/// in the report. Otherwise it is omitted to reduce output size.
pub fn build_report(
    scan: &ConflictScanResult,
    load_order: &[String],
    game_path: &Path,
    hashes: &HashMap<u64, String>,
    show_no_conflicts: bool,
    include_load_order: bool,
) -> ConflictReport {
    let resolve = |h: &u64| -> String {
        hashes
            .get(h)
            .cloned()
            .unwrap_or_else(|| h.to_string())
    };

    let resolve_archive = |ah: &u64| -> String {
        scan.archives
            .get(ah)
            .map(|a| a.file_name.clone())
            .unwrap_or_else(|| ah.to_string())
    };

    // Per-archive reports, in load-order
    let mut archive_reports = Vec::new();
    for (idx, name) in load_order.iter().enumerate() {
        let path = game_path.join(name);
        let key = fnv1a64_hash_path(&path);

        if let Some(vm) = scan.archives.get(&key) {
            // Always skip archives with no conflicts (matching GUI)
            if vm.wins.is_empty() && vm.loses.is_empty() {
                continue;
            }

            let winning_files: Vec<ConflictFileEntry> = vm
                .wins
                .iter()
                .map(|h| {
                    let conflicting: Vec<String> = scan
                        .conflicts
                        .get(h)
                        .map(|arcs| {
                            arcs.iter()
                                .filter(|a| **a != key)
                                .map(|a| resolve_archive(a))
                                .collect()
                        })
                        .unwrap_or_default();
                    ConflictFileEntry {
                        file: resolve(h),
                        conflicting_archives: conflicting,
                    }
                })
                .collect();

            let losing_files: Vec<ConflictFileEntry> = vm
                .loses
                .iter()
                .map(|h| {
                    let conflicting: Vec<String> = scan
                        .conflicts
                        .get(h)
                        .map(|arcs| {
                            arcs.iter()
                                .filter(|a| **a != key)
                                .map(|a| resolve_archive(a))
                                .collect()
                        })
                        .unwrap_or_default();
                    ConflictFileEntry {
                        file: resolve(h),
                        conflicting_archives: conflicting,
                    }
                })
                .collect();

            let status = if vm.wins.is_empty() && vm.files.len() == vm.loses.len() {
                "fully_overridden"
            } else {
                "has_conflicts"
            };

            let no_conflict_files = if show_no_conflicts {
                Some(
                    vm.get_no_conflicts()
                        .iter()
                        .map(|h| resolve(h))
                        .collect(),
                )
            } else {
                None
            };

            archive_reports.push(ArchiveReport {
                name: vm.file_name.clone(),
                load_order_index: idx,
                total_files: vm.files.len(),
                winning_count: vm.wins.len(),
                losing_count: vm.loses.len(),
                status: status.to_string(),
                winning_files,
                losing_files,
                no_conflict_files,
            });
        }
    }

    // Per-file conflicts
    let mut file_conflicts: Vec<FileConflict> = scan
        .conflicts
        .iter()
        .map(|(file_hash, archive_hashes)| {
            let archives_ordered: Vec<String> =
                archive_hashes.iter().map(|a| resolve_archive(a)).collect();
            let winner = archives_ordered.last().cloned().unwrap_or_default();
            FileConflict {
                file: resolve(file_hash),
                archives: archives_ordered,
                winner,
            }
        })
        .collect();
    file_conflicts.sort_by(|a, b| a.file.cmp(&b.file));

    ConflictReport {
        archive_path: game_path.to_string_lossy().into_owned(),
        total_archives: load_order.len(),
        total_conflicting_files: scan.conflicts.len(),
        load_order: if include_load_order {
            Some(load_order.to_vec())
        } else {
            None
        },
        archives: archive_reports,
        file_conflicts,
    }
}

/// Build a compact summary — one line per conflicting archive, no file details.
/// Designed to fit in an LLM context window even with thousands of mods.
pub fn build_summary(
    scan: &ConflictScanResult,
    load_order: &[String],
    game_path: &Path,
) -> SummaryReport {
    let mut summaries = Vec::new();

    for (idx, name) in load_order.iter().enumerate() {
        let path = game_path.join(name);
        let key = fnv1a64_hash_path(&path);

        if let Some(vm) = scan.archives.get(&key) {
            if vm.wins.is_empty() && vm.loses.is_empty() {
                continue;
            }

            let fully_overridden =
                vm.wins.is_empty() && vm.files.len() == vm.loses.len();

            summaries.push(ArchiveSummary {
                name: vm.file_name.clone(),
                load_order_index: idx,
                total_files: vm.files.len(),
                winning_count: vm.wins.len(),
                losing_count: vm.loses.len(),
                fully_overridden,
            });
        }
    }

    let fully_overridden_count = summaries.iter().filter(|a| a.fully_overridden).count();

    SummaryReport {
        archive_path: game_path.to_string_lossy().into_owned(),
        total_archives: load_order.len(),
        total_conflicting_files: scan.conflicts.len(),
        fully_overridden_count,
        archives: summaries,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn read_archive_vm(archive_file_path: &Path) -> Option<ArchiveViewModel> {
    match red4lib::archive::open_read(archive_file_path) {
        Ok(archive) => {
            let file_name = archive_file_path
                .file_name()
                .and_then(|f| f.to_str())
                .map(|s| s.to_owned())?;

            let mut hashes: Vec<u64> = archive.get_entries().clone().into_keys().collect();
            hashes.sort();

            Some(ArchiveViewModel {
                file_name,
                files: hashes,
                wins: Vec::new(),
                loses: Vec::new(),
            })
        }
        Err(e) => {
            log::warn!("Failed to read {}: {}", archive_file_path.display(), e);
            None
        }
    }
}

pub fn get_files(folder_path: &Path, extension: &str) -> Vec<PathBuf> {
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

pub fn read_file_to_vec(file_path: &Path) -> io::Result<Vec<String>> {
    let file = File::open(file_path)?;
    let reader = BufReader::new(file);
    Ok(reader.lines().map_while(Result::ok).collect())
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
