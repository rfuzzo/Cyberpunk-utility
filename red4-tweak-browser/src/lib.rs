#![warn(clippy::all, rust_2018_idioms)]

mod app;
pub use app::TemplateApp;
use log::info;
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use walkdir::{DirEntry, WalkDir};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TweakRecord {
    pub name: String,
    pub base: Option<String>,
    pub package: String,
    pub imports: Vec<String>,
}
impl TweakRecord {
    fn full_name(&self) -> String {
        format!("{}.{}", self.package, self.name)
    }
    fn possible_base_names(&self) -> Option<Vec<String>> {
        if self.base.is_none() {
            None
        } else {
            let mut results: Vec<String> = vec![];

            // possible is in packaege
            results.push(format!(
                "{}.{}",
                self.package,
                self.base.to_owned().unwrap()
            ));

            // and all imports
            for i in &self.imports {
                let n = format!("{}.{}", i, self.base.to_owned().unwrap());
                results.push(n);
            }

            Some(results)
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct TweakRecordVm {
    //pub full_name: String,
    pub children: Option<Vec<String>>,
    pub parent: Option<String>,
}

// The output is wrapped in a Result to allow matching on errors
// Returns an Iterator to the Reader of the lines of the file.
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn get_parents(vms: &HashMap<String, TweakRecordVm>, record: &str) -> Vec<String> {
    let mut result: Vec<String> = vec![];

    if let Some(b) = vms.get(record) {
        result.push(record.to_owned());
        if let Some(base) = &b.parent {
            let inner = get_parents(vms, base.as_str());
            for i in inner {
                result.push(i);
            }
        }
    }
    result
}

fn get_children_recursive(vms: &HashMap<String, TweakRecordVm>, record: &String) -> Vec<String> {
    let mut result: Vec<String> = vec![];

    if let Some(b) = vms.get(record) {
        if let Some(children) = &b.children {
            for child in children.iter() {
                result.push(child.to_owned());

                let inner_result = get_children_recursive(vms, child);
                for inner_child in inner_result {
                    result.push(inner_child);
                }
            }
        }
    }

    result.sort();
    result.dedup();
    result
}

pub fn get_hierarchy(records: Vec<TweakRecord>) -> HashMap<String, TweakRecordVm> {
    let mut vms: HashMap<String, TweakRecordVm> = HashMap::default();

    // add all records once
    for r in records.iter() {
        let v = TweakRecordVm {
            children: None,
            parent: if r.base.is_some() {
                Some("dummy".to_owned())
            } else {
                None
            },
        };
        vms.insert(r.full_name(), v);
    }

    // populate dependent data
    for r in records.iter().filter(|f| f.base.is_some()) {
        // fill in parent
        let mut parent_name: Option<String> = None;
        if let Some(base_names) = r.possible_base_names() {
            for base_name in &base_names {
                // try getting a real record
                if let Some(base) = vms.get_mut(base_name) {
                    // a base was found
                    // get children
                    if let Some(children) = &mut base.children {
                        children.push(r.full_name());
                    } else {
                        base.children = Some(vec![r.full_name()]);
                    }
                    parent_name = Some(base_name.to_string());
                    break;
                }
            }
        }

        // save parent in vm
        if let Some(this_vm) = vms.get_mut(&r.full_name()) {
            this_vm.parent = parent_name.clone();
        }
    }

    vms
}

fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub fn get_records(path: &PathBuf) -> Vec<TweakRecord> {
    let mut records: Vec<TweakRecord> = vec![];
    let mut parsed = 0;
    let mut total = 0;

    for entry in WalkDir::new(path)
        .into_iter()
        .filter_entry(|e| !is_hidden(e))
        .filter_map(|e| e.ok())
    {
        let filename = entry.path();
        if filename.is_dir() {
            continue;
        }
        if let Some(ext) = filename.extension() {
            if !ext.to_string_lossy().eq("tweak") {
                continue;
            }
        }

        // parse each file
        total += 1;
        let mut ignore = false;
        let mut package = "".to_owned();
        let mut imports: Vec<String> = vec![];

        if let Ok(lines) = read_lines(entry.path()) {
            parsed += 1;
            for line in lines.flatten() {
                if line.starts_with("package ") {
                    package = line["package".len() + 1..].to_string();

                    continue;
                }
                if line.starts_with("using ") {
                    let usings = line["using".len() + 1..].to_string();
                    let u_splits = usings.split(", ").collect::<Vec<_>>();
                    imports = u_splits
                        .into_iter()
                        .map(|f| f.to_owned())
                        .collect::<Vec<_>>();
                    continue;
                }
                if line.contains('=') {
                    continue;
                }
                if line.starts_with("[ ") && line.contains(']') {
                    continue;
                }
                if line.starts_with('[') && !line.contains(']') {
                    ignore = true;
                    continue;
                }
                if line.is_empty() {
                    continue;
                }
                // props
                if line.starts_with('{') {
                    ignore = true;
                    continue;
                }
                if line.starts_with('}') {
                    ignore = false;
                    continue;
                }
                if line.starts_with(']') {
                    ignore = false;
                    continue;
                }
                if ignore {
                    continue;
                }

                let mut name = line.to_owned();
                let mut base = None;
                if name.contains(" : ") {
                    let splits = line.split(" : ").collect::<Vec<_>>();
                    name = splits.first().unwrap().to_string();
                    base = Some(splits.last().unwrap().to_string());
                }

                let record: TweakRecord = TweakRecord {
                    name,
                    base,
                    package: package.to_owned(),
                    imports: imports.to_owned(),
                };
                if !records.contains(&record) {
                    records.push(record);
                    //println!("Adding record {}", &line);
                }
            }

            info!("Parsed {} files", parsed);
        }
    }

    info!("Parsed {}/{} files", parsed, total);
    records
}
