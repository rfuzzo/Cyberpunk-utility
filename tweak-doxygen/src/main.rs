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

fn get_lines(path: &str) -> HashMap<String, Vec<String>> {
    let mut map: HashMap<String, Vec<String>> = HashMap::new();

    for file in WalkDir::new(path)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.file_type().is_dir())
    {
        // filter to tweak files
        if !file
            .path()
            .extension()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("tweak")
        {
            //println!("ERROR: Not a tweak file {}", file.path().display());
            continue;
        }

        //println!("Processing {} ...", file.path().display());

        let mut ns = String::from("");
        let mut inner_lines = vec![];

        // read lines in file
        let reader = BufReader::new(fs::File::open(file.path()).unwrap());
        for line in reader.lines() {
            let f = file.path().display().to_string();
            let line = line.unwrap_or_else(|_| panic!("ERROR at {f}"));

            if line.starts_with("package ") {
                ns = line.clone()[8..].trim_end().to_string();
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
        if ns.is_empty() {
            println!("ERROR: no namespace in file {}", file.path().display());
            continue;
        }

        // add to map
        if !map.contains_key(&ns) {
            map.insert(ns.to_string(), vec![]);
        }

        for element in inner_lines {
            map.get_mut(&ns).unwrap().push(element);
        }
    }

    map
}

fn convert_to_cs(map: &mut HashMap<String, Vec<String>>, out_path: &str) {
    // check lowercase duplicates
    let mut check = vec![];

    //for key in map.keys().sorted() {}
    for (ns, classes) in map {
        let mut outpath = Path::new(&out_path);
        if check.contains(&ns.to_lowercase()) {
            println!("DUPLICATE {ns} ...");

            outpath = Path::new("Z:\\_out\\in\\duplicates");
            fs::create_dir_all(outpath).expect("Error creating folder");
        }

        //println!("Processing {ns} ...");

        let file = File::create(outpath.join(format!("{ns}.cs")))
            .unwrap_or_else(|_| panic!("ERROR: Failed to create file {ns}"));
        let mut writer = BufWriter::new(file);

        for line in classes.iter_mut() {
            line.insert_str(0, "public class ");
            line.push_str(" {}");
        }

        // write to file
        let namespace = format!("namespace {ns};");
        writer.write_all(namespace.as_bytes()).unwrap();
        writer.write_all(b"\n").unwrap();
        writer.write_all(b"\n").unwrap();

        for line in classes.iter() {
            writer.write_all(line.as_bytes()).unwrap();
            writer.write_all(b"\n").unwrap();
        }

        check.push(ns.to_lowercase());
    }
}
