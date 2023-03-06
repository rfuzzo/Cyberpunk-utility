use clap::{Parser, Subcommand};
use cmop::{gather_mods, get_mods_from_rules, parse_rules, read_file_as_list, topo_sort};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Lists the current mod load order
    List {
        /// Root game folder ("Cyberpunk 2077"). Default is current working directory
        #[arg(default_value_t = String::from("./"))]
        root: String,
    },
    /// Sorts the current mod load order according to specified rules
    Sort {
        /// Root game folder ("Cyberpunk 2077"). Default is current working directory
        #[arg(default_value_t = String::from("./"))]
        root: String,

        /// Folder to read sorting rules from. Default is ./cmop
        #[arg(short, long, default_value_t = String::from("./cmop"))]
        rules: String,

        /// Just print the suggested load order without sorting
        #[arg(short, long)]
        dry_run: bool,

        /// Read the input mods from a file instead of checking the root folder
        #[arg(short, long)]
        mod_list: Option<PathBuf>,
    },
    /// Verifies integrity of the specified rules
    Verify {
        /// Folder to read sorting rules from. Default is ./cmop
        #[arg(short, long, default_value_t = String::from("./cmop"))]
        rules: String,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::List { root }) => list_mods(&root.into()),
        Some(Commands::Verify { rules }) => verify(&rules.into()),
        Some(Commands::Sort {
            root,
            rules,
            mod_list,
            dry_run,
        }) => sort(&root.into(), &rules.into(), mod_list, *dry_run),
        None => ExitCode::FAILURE,
    }
}

/// Sorts the current mod load order according to specified rules
fn sort(
    root: &PathBuf,
    rules_path: &PathBuf,
    mod_list: &Option<PathBuf>,
    dry_run: bool,
) -> ExitCode {
    // gather mods (optionally from a list)
    let mods: Vec<String>;
    if let Some(modlist_path) = mod_list {
        mods = read_file_as_list(modlist_path);
    } else {
        match gather_mods(root) {
            Ok(m) => mods = m,
            Err(e) => {
                println!("No mods found: {e}");
                return ExitCode::FAILURE;
            }
        }
    }

    match parse_rules(rules_path) {
        Ok(rules) => match topo_sort(mods, &rules) {
            Ok(result) => {
                if dry_run {
                    println!("Dry run...");
                    println!("{result:?}");
                } else {
                    println!("Sorting mods...");
                    println!("{result:?}");

                    todo!()
                }

                ExitCode::SUCCESS
            }
            Err(e) => {
                println!("error sorting: {e:?}");
                ExitCode::FAILURE
            }
        },
        Err(e) => {
            println!("error parsing file: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Verifies integrity of the specified rules
fn verify(rules_path: &PathBuf) -> ExitCode {
    //println!("Verifying rules from {} ...", rules_path.display());

    match parse_rules(rules_path) {
        Ok(rules) => {
            let mods = get_mods_from_rules(&rules);
            match topo_sort(mods, &rules) {
                Ok(_) => {
                    println!("true");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    println!("false");
                    ExitCode::FAILURE
                }
            }
        }
        Err(e) => {
            println!("error parsing file: {e:?}");
            ExitCode::FAILURE
        }
    }
}

/// Lists the current mod load order
fn list_mods(root: &PathBuf) -> ExitCode {
    //println!("Printing active mods...");

    match gather_mods(root) {
        Ok(mods) => {
            for m in mods {
                println!("{}", m);
            }

            ExitCode::SUCCESS
        }
        _ => ExitCode::FAILURE,
    }
}
