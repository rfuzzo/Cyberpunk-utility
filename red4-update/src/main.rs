use std::{
    collections::HashMap,
    env,
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

use clap::{Parser, Subcommand};
use colored::*;
use red4_update::{get_info, Diff, FileInfo};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// generate patch diff
    Generate {
        /// Path to old Game folder
        #[arg(short, long)]
        old_dir: PathBuf,

        /// Path to new Game folder
        #[arg(short, long)]
        new_dir: PathBuf,
    },
    /// checks a mod archive
    Check {
        /// Path to a folder with archives to check
        path: Option<PathBuf>,
    },
}

fn main() {
    simple_logger::init().unwrap();

    let cli = Cli::parse();

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Generate { old_dir, new_dir }) => {
            let diff = generate_diff(old_dir, new_dir);

            // write
            log::info!("Creating json ...");
            if let Ok(json) = serde_json::to_string_pretty(&diff) {
                // to file
                let mut file = File::create("diff.json").expect("Failed to create file");
                file.write_all(json.as_bytes())
                    .expect("Failed to write file");
            } else {
                log::error!("Failed to serialize diff.")
            }
        }
        Some(Commands::Check { path: path_option }) => {
            let mut path = PathBuf::from("");
            if let Some(p) = path_option {
                path = p.to_path_buf();
            }
            if !path.exists() {
                if let Ok(cwd) = env::current_dir() {
                    path = cwd;
                } else {
                    log::error!("No input path found");
                    return;
                }
            }

            check_path_for_updates(path);

            println!("Press any button to continue ...");
            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .expect("error: unable to read user input");
        }
        None => {}
    }
}

fn check_path_for_updates(path: PathBuf) {
    log::info!("Loading Hashes ...");
    let hash_map = red4lib::get_red4_hashes();

    log::info!("Loading Diff ...");
    let bytes = include_bytes!("diff.json");
    let diff: Diff = serde_json::from_slice(bytes).expect("Could not deserialize diff");

    log::info!("Parsing mods ...");
    let mut check_map: HashMap<u64, FileInfo> = HashMap::default();
    get_info(&path, &mut check_map, &hash_map);

    log::info!("Creating report ...");
    let mut report: HashMap<String, Diff> = HashMap::default();
    for (hash, file_info) in check_map {
        let archive = file_info.archive_name.clone();
        if !report.contains_key(&archive) {
            let d = Diff::default();
            report.insert(archive.clone(), d);
        }

        if let Some(d) = report.get_mut(&archive) {
            if diff.deleted.contains_key(&hash) {
                d.deleted.insert(hash, file_info.clone());
            }
            if diff.added.contains_key(&hash) {
                d.added.insert(hash, file_info.clone());
            }
            if diff.changed.contains_key(&hash) {
                d.changed.insert(hash, file_info.clone());
            }
        }
    }

    log::info!("Creating json ...");
    if let Ok(json) = serde_json::to_string_pretty(&report) {
        // to file
        let mut file = File::create("report.json").expect("Failed to create file");
        file.write_all(json.as_bytes())
            .expect("Failed to write file");
    } else {
        log::error!("Failed to serialize report.")
    }

    // report to console
    println!();
    println!("The following mods may need to be updated:");
    let mut keys = report.keys().collect::<Vec<_>>();
    keys.sort();
    for key in keys {
        if let Some(info) = report.get(key) {
            if info.deleted.len() + info.added.len() + info.changed.len() == 0 {
                continue;
            }

            println!();
            println!("{}", key.blue().bold());
            if !info.deleted.is_empty() {
                println!("\t{}", "deleted:".red());
                for i in info.deleted.values() {
                    println!("\t\t{}", i.name);
                }
            }

            if !info.added.is_empty() {
                println!("\t{}", "added:".green());
                for i in info.added.values() {
                    println!("\t\t{}", i.name);
                }
            }

            if !info.changed.is_empty() {
                println!("\t{}", "changed:".yellow());
                for i in info.changed.values() {
                    println!("\t\t{}", i.name);
                }
            }
        }
    }
}

fn generate_diff(old_dir: &Path, new_dir: &Path) -> Diff {
    log::info!("Loading Hashes ...");
    let hash_map = red4lib::get_red4_hashes();

    let old_map_path = PathBuf::from("old_file_map.json");
    let new_map_path = PathBuf::from("new_file_map.json");

    // assume this is a game folder
    let mut old_file_map: HashMap<u64, FileInfo> = HashMap::default();
    if old_map_path.exists() {
        let json = fs::read_to_string(old_map_path).expect("ould not read old map");
        old_file_map = serde_json::from_str(json.as_str()).expect("Could not deserialize old map");
        log::info!("Read old map from file");
    } else {
        get_info(
            &old_dir.join("archive").join("pc").join("content"),
            &mut old_file_map,
            &hash_map,
        );
        get_info(
            &old_dir.join("archive").join("pc").join("ep1"),
            &mut old_file_map,
            &hash_map,
        );

        log::info!("Creating old_file_map ...");
        if let Ok(json) = serde_json::to_string_pretty(&old_file_map) {
            // to file
            let mut file = File::create(old_map_path).expect("Failed to create file");
            file.write_all(json.as_bytes())
                .expect("Failed to write file");
        } else {
            log::error!("Failed to serialize diff.")
        }
    }

    // assume this is a game folder
    let mut new_file_map: HashMap<u64, FileInfo> = HashMap::default();
    if new_map_path.exists() {
        let json = fs::read_to_string(new_map_path).expect("ould not read new map");
        new_file_map = serde_json::from_str(json.as_str()).expect("Could not deserialize new map");
        log::info!("Read new map from file");
    } else {
        get_info(
            &new_dir.join("archive").join("pc").join("content"),
            &mut new_file_map,
            &hash_map,
        );
        get_info(
            &new_dir.join("archive").join("pc").join("ep1"),
            &mut new_file_map,
            &hash_map,
        );

        log::info!("Creating new_file_map ...");
        if let Ok(json) = serde_json::to_string_pretty(&new_file_map) {
            // to file
            let mut file = File::create(new_map_path).expect("Failed to create file");
            file.write_all(json.as_bytes())
                .expect("Failed to write file");
        } else {
            log::error!("Failed to serialize diff.")
        }
    }

    // diff the maps
    log::info!("Checking deleted files ...");
    let deleted_vec: Vec<FileInfo> = old_file_map
        .iter()
        .filter(|(k, _v)| !new_file_map.contains_key(k))
        .map(|f| f.1.clone())
        .collect();
    log::info!("Found {} deleted files", deleted_vec.len());

    log::info!("Checking added files ...");
    let added_vec: Vec<FileInfo> = new_file_map
        .iter()
        .filter(|(k, _v)| !old_file_map.contains_key(k))
        .map(|f| f.1.clone())
        .collect();
    log::info!("Found {} added files", added_vec.len());

    log::info!("Checking changed files ...");
    let changed_vec: Vec<FileInfo> = old_file_map
        .iter()
        .filter(|(k, old)| {
            if let Some(new) = new_file_map.get(k) {
                if old.sha1 != new.sha1 {
                    return true;
                }
            }
            false
        })
        .map(|f| f.1.clone())
        .collect();
    log::info!("Found {} changed files", changed_vec.len());

    let mut deleted: HashMap<u64, FileInfo> = HashMap::default();
    for i in deleted_vec {
        deleted.insert(i.hash, i);
    }
    let mut added: HashMap<u64, FileInfo> = HashMap::default();
    for i in added_vec {
        added.insert(i.hash, i);
    }
    let mut changed: HashMap<u64, FileInfo> = HashMap::default();
    for i in changed_vec {
        changed.insert(i.hash, i);
    }

    Diff {
        deleted,
        added,
        changed,
    }
}
