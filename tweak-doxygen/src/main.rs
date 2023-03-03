use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = env::args().collect();

    let mut in_path = env::current_dir().expect("No working directory");

    if args.len() > 1 {
        in_path = (&args[1]).into();
    }

    let mut out_path = PathBuf::from("./_tweak_cs");
    if args.len() > 2 {
        out_path = (&args[2]).into();
    }

    let mut map = get_lines(in_path.as_os_str().to_str().expect("no usable input path"));
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
        .filter(|e| e.path().extension().is_some())
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
        let mut skipping = false;
        let reader = BufReader::new(fs::File::open(file.path()).unwrap());
        for (linen, rawline) in reader.lines().enumerate() {
            let f = file.path().display().to_string();
            let mut line = rawline.unwrap_or_else(|_| panic!("ERROR at {f}:{linen}"));

            // end block comments
            if skipping {
                if line.starts_with("*/") {
                    skipping = false;
                    // still continue with rest of line
                    line = line[2..].to_string();
                } else if line.ends_with("*/") {
                    skipping = false;
                    // can be skipped
                    continue;
                } else if line.contains("/*") && line.contains("*/") {
                    // do nothing
                    println!("???")
                } else if line.contains("*/") {
                    skipping = false;
                    // still continue with rest of line
                    let idx = line.find("*/").unwrap() + 2;
                    line = line[idx..].to_string();
                }

                // nothing found to end skipping
                if skipping {
                    continue;
                }
            }

            // start block comments
            if !skipping {
                if line.starts_with("/*") && !line.contains("*/") {
                    skipping = true;
                    // can be skipped and continue skipping
                    continue;
                } else if line.starts_with("/*") && line.contains("*/") {
                    // block comment in one line
                    // do nothing but ignore the comment
                    // still continue with rest of line
                    let idx = line.find("*/").unwrap() + 2;
                    line = line[idx..].to_string();
                } else if line.contains("/*") && !line.contains("*/") {
                    skipping = true;

                    // println!(
                    //     "WARNING: blockcomment across multiple lines at {} in {}",
                    //     linen,
                    //     file.path().display()
                    // );

                    // still evaluate start of the line
                    let idx = line.find("/*").unwrap();
                    line = line[..idx].to_string();
                }
            }

            // namespaces
            if line.starts_with("package ") {
                namespace = line.clone()["package ".len()..].trim_end().to_string();
            }

            // usings
            if let Some(stripped) = line.strip_prefix("using ") {
                usings = stripped
                    //.trim_end()
                    .split(',')
                    .map(|s| s.trim().to_owned())
                    .collect::<Vec<_>>();
            }

            // skip specific lines
            if line.is_empty()
                || line.starts_with('\t')
                || line.starts_with(' ')
                || line.starts_with('{')
                || line.starts_with('}')
                || line.starts_with('[')
                || line.starts_with(']')
                || line.starts_with("package ")
                || line.starts_with("using ")
                || line.starts_with("//")
                || line.contains('=')
            {
                continue;
            }

            // classes
            inner_lines.push(line);
        }

        // error if no namespace
        if namespace.is_empty() {
            println!("ERROR: no namespace in file {}", file.path().display());
            continue;
        }

        // add to map
        // add namespace
        if !map.contains_key(&namespace) {
            map.insert(namespace.to_string(), PackageInfo::default());
        }
        // add class to namespace
        for c in inner_lines {
            // need to sanitze class names e.g. OverrideAuthorizationClassHack : DeviceQuickHack // ---> obsolete
            // only take stuff before the first comment
            let mut class = c.clone();
            if c.contains("//") {
                // clean comments
                let s: String = c.split("//").take(1).collect();
                //println!("INFO: sanitize [{}] in {}", c, file.path().display());
                class = s.trim_end().to_string();
            }

            if !map[&namespace].classes.contains(&class) {
                map.get_mut(&namespace).unwrap().classes.push(class);
            }
        }
        // add usings to namespace
        for u in usings {
            if !map[&namespace].usings.contains(&u.to_string()) {
                map.get_mut(&namespace).unwrap().usings.push(u.to_string());
            }
        }
    }

    map
}

fn convert_to_cs(map: &mut HashMap<String, PackageInfo>, out_path: PathBuf) {
    // check lowercase duplicates
    let mut check = vec![];

    println!("Starting printing ...");
    let outpath = Path::new(&out_path);
    fs::create_dir_all(outpath).expect("Error creating folder");

    for (key, package) in map {
        if check.contains(&key.to_lowercase()) {
            panic!("DUPLICATE {key} ...");
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
