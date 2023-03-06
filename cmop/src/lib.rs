use std::fs::{self, File};
use std::io::BufRead;
use std::io::{self};
use std::path::Path;
use topological_sort::TopologicalSort;

#[derive(Default)]
pub struct Rules {
    pub order: Vec<(String, String)>,
}

enum RuleKind {
    None,
    Order,
    //Note,
}

// sort the strings according to pairs
pub fn topo_sort(input: &Vec<String>, rules: &Rules) -> Result<Vec<String>, &'static str> {
    // Create a new TopologicalSort instance
    let mut sort = TopologicalSort::<&str>::new();

    // Add all the strings as items
    for s in input {
        sort.insert(s.as_ref());
    }

    // Add all the pairs as dependencies
    for (a, b) in &rules.order {
        if input.contains(a) && input.contains(b) {
            sort.add_dependency(a.as_ref(), b.as_ref());
        }
    }

    // Sort the items and collect them into a vector
    let mut result: Vec<String> = Vec::new();
    while let Some(s) = sort.pop() {
        result.push(s.into());
    }

    if !sort.is_empty() {
        return Err("cycle detected");
    }

    // Return the sorted vector
    Ok(result)
}

// Returns an Iterator to the Reader of the lines of the file.
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

// custom rules parser
pub fn parse_rules<P>(rules_dir: P) -> Result<Rules, &'static str>
where
    P: AsRef<Path>,
{
    let mut rules: Rules = Rules::default();
    let mut order: Vec<(String, String)> = vec![];

    let mut orders: Vec<Vec<String>> = vec![];

    // todo scan directory for user files
    let rules_path = rules_dir.as_ref().join("cmop_rules_base.txt");
    if let Ok(lines) = read_lines(rules_path) {
        let mut parsing = false;
        let mut current_order: Vec<String> = vec![];
        let mut current_rule: RuleKind = RuleKind::None;

        // Consumes the iterator, returns an (Optional) String
        for line in lines.flatten() {
            // parse each line
            if line.starts_with(';') {
                continue;
            }

            // order parsing
            if parsing && line.is_empty() {
                parsing = false;
                match current_rule {
                    RuleKind::Order => {
                        orders.push(current_order.to_owned());
                        current_order.clear();
                    }
                    //RuleKind::Note => todo!(),
                    RuleKind::None => todo!(),
                }

                continue;
            }

            if !parsing && line == "[Order]" {
                parsing = true;
                current_rule = RuleKind::Order;
                continue;
            }

            if parsing {
                match current_rule {
                    RuleKind::Order => current_order.push(line),
                    //RuleKind::Note => todo!(),
                    RuleKind::None => todo!(),
                }
            }
        }
        orders.push(current_order.to_owned());

        // process orders
        for o in orders {
            match o.len().cmp(&2) {
                std::cmp::Ordering::Less => continue,
                std::cmp::Ordering::Equal => order.push((o[0].to_owned(), o[1].to_owned())),
                std::cmp::Ordering::Greater => {
                    // add all pairs
                    for i in 0..o.len() - 1 {
                        order.push((o[i].to_owned(), o[i + 1].to_owned()));
                    }
                }
            }
        }

        // set data
        rules.order = order;

        Ok(rules)
    } else {
        Err("failed reading file")
    }
}

// read file line by line into vector
pub fn read_file_as_list<P>(modlist_path: P) -> Vec<String>
where
    P: AsRef<Path>,
{
    let mut result: Vec<String> = vec![];
    if let Ok(lines) = read_lines(modlist_path) {
        for line in lines.flatten() {
            result.push(line);
        }
    }
    result
}

pub fn get_mods_from_rules(rules: &Rules) -> Vec<String> {
    let mut result: Vec<String> = vec![];
    for r in rules.order.iter() {
        let mut a = r.0.to_owned();
        if !result.contains(&a) {
            result.push(a);
        }
        a = r.1.to_owned();
        if !result.contains(&a) {
            result.push(a);
        }
    }
    result
}

pub fn verify_rules() {}

pub fn gather_mods<P>(root: &P) -> io::Result<Vec<String>>
where
    P: AsRef<Path>,
{
    // gather mods from archive/pc/mod
    let archive_path = root.as_ref().join("archive").join("pc").join("mod");
    let mut entries = fs::read_dir(archive_path)?
        .map(|res| res.map(|e| e.path()))
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| !e.is_dir())
        .filter(|e| e.extension().is_some())
        .filter(|e| e.extension().unwrap().to_str().unwrap().contains("archive"))
        .map(|e| String::from(e.file_name().unwrap().to_str().unwrap()))
        .collect::<Vec<_>>();

    // gather mods from mods/<NAME>

    entries.sort();

    Ok(entries)
}
