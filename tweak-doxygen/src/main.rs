use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::{BufWriter, Write};
use std::path::Path;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.is_empty() {
        println!("please provide an input path");
    }

    let in_path = &args[1];
    let mut out_path = "";
    if args.len() > 1 {
        out_path = &args[2];
    }
    if out_path.is_empty() {
        out_path = "html";
    }

    let mut map = get_lines(in_path.as_str());
    convert_to_cs(&mut map, out_path);

    println!("Done!");
}

#[derive(Default)]
struct PackageInfo {
    classes: Vec<String>,
    usings: Vec<String>,
}

fn get_lines(path: &str) -> HashMap<String, PackageInfo> {
    let mut map: HashMap<String, PackageInfo> = HashMap::new();

    println!("Starting collecting ...");

    for file in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
        .filter(|e| {
            e.path()
                .extension()
                .unwrap()
                .to_str()
                .unwrap()
                .contains("tweak")
        })
    {
        //println!("Processing {} ...", file.path().display());

        let mut namespace = String::from("");
        let mut usings: Vec<String> = vec![];
        let mut inner_lines = vec![];

        // read lines in file
        let reader = BufReader::new(fs::File::open(file.path()).unwrap());
        for line in reader.lines() {
            let f = file.path().display().to_string();
            let line = line.unwrap_or_else(|_| panic!("ERROR at {f}"));

            if line.starts_with("package ") {
                namespace = line.clone()["package ".len()..].trim_end().to_string();
            }

            if let Some(stripped) = line.strip_prefix("using ") {
                usings = stripped
                    //.trim_end()
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .collect::<Vec<_>>();
            }

            if !line.is_empty()
                && !line.starts_with('\t')
                && !line.starts_with(' ')
                && !line.starts_with('{')
                && !line.starts_with('}')
                && !line.starts_with('[')
                && !line.starts_with(']')
                && !line.starts_with("package ")
                && !line.starts_with("using ")
                && !line.starts_with("//")
                && !line.contains('=')
            {
                inner_lines.push(line);
            }
        }

        // error if no namespace
        if namespace.is_empty() {
            println!("ERROR: no namespace in file {}", file.path().display());
            continue;
        }

        // add to map
        if !map.contains_key(&namespace) {
            map.insert(namespace.to_string(), PackageInfo::default());
        }
        for c in inner_lines {
            if !map[&namespace].classes.contains(&c) {
                map.get_mut(&namespace).unwrap().classes.push(c);
            }
        }
        for u in usings {
            if !map[&namespace].usings.contains(&u.to_string()) {
                map.get_mut(&namespace).unwrap().usings.push(u.to_string());
            }
        }
    }

    map
}

fn convert_to_cs(map: &mut HashMap<String, PackageInfo>, out_path: &str) {
    // check lowercase duplicates
    let mut check = vec![];

    println!("Starting printing ...");

    for (key, package) in map {
        let mut outpath = Path::new(&out_path);
        if check.contains(&key.to_lowercase()) {
            println!("DUPLICATE {key} ...");

            outpath = Path::new("Z:\\_out\\in\\duplicates");
            fs::create_dir_all(outpath).expect("Error creating folder");
        }

        //println!("Processing {key} ...");

        let file = File::create(outpath.join(format!("{key}.cs")))
            .unwrap_or_else(|_| panic!("ERROR: Failed to create file {key}"));
        let mut writer = BufWriter::new(file);

        // write to file
        // usings //using System.Collections.Generic;
        for u in package.usings.iter() {
            writer
                .write_all(format!("using {u};\n").as_bytes())
                .unwrap();
        }

        // namespace
        writer
            .write_all(format!("namespace {key};\n\n").as_bytes())
            .unwrap();

        // classes
        for c in package.classes.iter() {
            writer
                .write_all(format!("public class {c} {{ }}\n").as_bytes())
                .unwrap();
        }

        check.push(key.to_lowercase());
    }
}
