use cl_optimizer::{parse_rules, read_file_as_list, topo_sort};
use std::env;

fn main() {
    // Get file paths from command line arguments
    let mut args = env::args().skip(1);
    let modlist_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Missing modlist file path");
            return;
        }
    };
    let rules_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Missing rules file path");
            return;
        }
    };

    let mods = read_file_as_list(&modlist_path);

    match parse_rules(&rules_path) {
        Ok(rules) => match topo_sort(mods, &rules) {
            Ok(result) => println!("{:?}", result),
            Err(e) => println!("error sorting: {e:?}"),
        },
        Err(e) => println!("error parsing file: {e:?}"),
    }
}
