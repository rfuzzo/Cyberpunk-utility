use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug, serde::Serialize, serde::Deserialize, Default)]
pub struct Diff {
    pub deleted: HashMap<u64, FileInfo>,
    pub added: HashMap<u64, FileInfo>,
    pub changed: HashMap<u64, FileInfo>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Default, Clone)]
pub struct FileInfo {
    pub hash: u64,
    pub name: String,
    pub archive_name: String,
    pub sha1: String,
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

pub fn get_info(
    archives_path: &Path,
    file_map: &mut HashMap<u64, FileInfo>,
    hash_map: &HashMap<u64, String>,
) {
    // TODO ignore lang option
    let files = get_files(archives_path, "archive");
    let files_content = files.iter().filter(|f| {
        if let Some(file_name) = f.file_name() {
            return !file_name.to_string_lossy().starts_with("lang_");
        }
        false
    });
    for path in files_content {
        log::info!("Parsing {}", &path.display());
        if let Ok(archive) = red4lib::archive::open_read(path) {
            let archive_name = path
                .file_name()
                .unwrap()
                .to_ascii_lowercase()
                .to_str()
                .unwrap()
                .to_owned();

            for (hash, entry) in archive.get_entries() {
                let mut name = hash.to_string();
                if let Some(resolved_name) = hash_map.get(hash) {
                    name = resolved_name.to_owned();
                }

                let mut sha = "".to_owned();
                for d in entry.entry.sha1_hash() {
                    sha += format!("{:x}", d).as_str();
                }

                let entry = FileInfo {
                    hash: *hash,
                    name,
                    archive_name: archive_name.to_owned(),
                    sha1: sha,
                };
                file_map.insert(*hash, entry);
            }
        }
    }
}
