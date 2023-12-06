use std::{collections::HashMap, path::Path};

use red4lib::{archive::Archive, get_files};

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
        if let Ok(archive) = Archive::from_file(path) {
            let archive_name = path
                .file_name()
                .unwrap()
                .to_ascii_lowercase()
                .to_str()
                .unwrap()
                .to_owned();

            let archive_files = archive
                .index
                .file_entries
                .iter()
                .map(|f| {
                    let hash = *f.0;
                    let mut sha = "".to_owned();
                    for d in f.1.sha1_hash {
                        sha += format!("{:x}", d).as_str();
                    }
                    (hash, sha)
                })
                .collect::<Vec<_>>();
            for (hash, sha1) in archive_files {
                let mut name = hash.to_string();
                if let Some(resolved_name) = hash_map.get(&hash) {
                    name = resolved_name.to_owned();
                }
                let entry = FileInfo {
                    hash,
                    name,
                    archive_name: archive_name.to_owned(),
                    sha1,
                };
                file_map.insert(hash, entry);
            }
        }
    }
}
