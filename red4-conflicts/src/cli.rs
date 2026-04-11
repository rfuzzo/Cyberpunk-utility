use std::path::{Path, PathBuf};

use clap::Parser;
use regex::RegexBuilder;
use red4_conflicts::scanner;

#[derive(Parser)]
#[command(
    name = "red4-conflicts-cli",
    version,
    about = "Detect file-level conflicts between Cyberpunk 2077 mod archives and output a structured JSON report.",
    after_long_help = "\
EXAMPLES:
    # Auto-detect game path and output compact JSON
    red4-conflicts-cli

    # Specify path explicitly
    red4-conflicts-cli \"C:\\Games\\Cyberpunk 2077\\archive\\pc\\mod\"

    # Pretty-print and save to file
    red4-conflicts-cli --pretty > conflicts.json

    # Output in TOON format (compact, optimized for LLM token usage)
    red4-conflicts-cli --toon > conflicts.txt

    # Filter to a specific mod
    red4-conflicts-cli --mod-filter \"ArchiveXL\"

    # Filter conflicting files by regex (e.g. all .mesh files)
    red4-conflicts-cli --file-filter \"\\.mesh$\"

    # Combine filters
    red4-conflicts-cli --mod-filter \"hair\" --file-filter \"\\.mesh$\" --pretty"
)]
struct Cli {
    /// Path to the folder containing .archive mod files.
    ///
    /// If omitted, the tool auto-detects the Cyberpunk 2077 installation
    /// by searching standard Steam and GOG install locations:
    ///
    ///   Steam:  C:\Program Files (x86)\Steam\steamapps\common\Cyberpunk 2077
    ///   Steam:  D:\SteamLibrary\steamapps\common\Cyberpunk 2077  (D-G drives)
    ///   GOG:    C:\Program Files (x86)\GOG Galaxy\Games\Cyberpunk 2077
    ///   GOG:    C:\GOG Games\Cyberpunk 2077
    ///
    /// The path should point to the "archive\pc\mod" folder inside the game directory.
    archive_path: Option<PathBuf>,

    /// Pretty-print the JSON output.
    #[arg(long, default_value_t = false)]
    pretty: bool,

    /// Output in TOON format (Token Oriented Object Notation) instead of JSON.
    /// Experimental — uses ~40% fewer tokens than JSON, optimized for LLM input.
    /// When enabled, --pretty is ignored.
    #[arg(long, default_value_t = false)]
    toon: bool,

    /// Filter archives by name. Plain strings match as substrings (case-insensitive).
    /// Regex syntax is also supported (e.g. "^ArchiveXL" or ".*hair.*").
    #[arg(long, value_name = "PATTERN")]
    mod_filter: Option<String>,

    /// Filter conflicting file entries by path. Plain strings match as substrings
    /// (case-insensitive). Regex syntax is also supported (e.g. "\\.mesh$").
    #[arg(long, value_name = "PATTERN")]
    file_filter: Option<String>,

    /// Output a compact summary instead of the full report.
    /// One line per conflicting archive with win/loss counts and a
    /// fully_overridden flag for mods that have zero effect in-game.
    /// Use --mod-filter to drill into specific mods for full details.
    #[arg(long, default_value_t = false)]
    summary: bool,

    /// Include non-conflicting files in each archive's report.
    /// By default, only winning and losing files are shown (matching the GUI default).
    #[arg(long, default_value_t = false)]
    show_no_conflicts: bool,

    /// Include the full load order list in the output.
    /// By default it is omitted to reduce output size — each archive already
    /// includes its load_order_index for ordering context.
    #[arg(long, default_value_t = false)]
    show_load_order: bool,
}

/// Search standard Cyberpunk 2077 install locations and return the mod archive path.
fn detect_game_path() -> Option<PathBuf> {
    let suffix = Path::new("Cyberpunk 2077")
        .join("archive")
        .join("pc")
        .join("mod");

    let mut candidates: Vec<PathBuf> = vec![
        // Steam default
        PathBuf::from(r"C:\Program Files (x86)\Steam\steamapps\common").join(&suffix),
        PathBuf::from(r"C:\Program Files\Steam\steamapps\common").join(&suffix),
        // GOG
        PathBuf::from(r"C:\Program Files (x86)\GOG Galaxy\Games").join(&suffix),
        PathBuf::from(r"C:\GOG Games").join(&suffix),
    ];

    // Steam libraries on common drive letters
    for drive in ['D', 'E', 'F', 'G'] {
        candidates.push(
            PathBuf::from(format!("{}:\\SteamLibrary\\steamapps\\common", drive)).join(&suffix),
        );
    }

    candidates.into_iter().find(|p| p.exists())
}

fn main() {
    let _ = simple_logger::init_with_level(log::Level::Warn);

    let cli = Cli::parse();

    // Resolve archive path
    let archive_path = match cli.archive_path {
        Some(path) => {
            if !path.exists() {
                eprintln!("Error: path does not exist: {}", path.display());
                std::process::exit(1);
            }
            path
        }
        None => match detect_game_path() {
            Some(path) => {
                eprintln!("Auto-detected game path: {}", path.display());
                path
            }
            None => {
                eprintln!("Error: could not auto-detect Cyberpunk 2077 installation.");
                eprintln!("Please provide the path to your archive/pc/mod folder as an argument.");
                eprintln!("Run with --help for details.");
                std::process::exit(1);
            }
        },
    };

    // Compile filters
    let mod_regex = cli.mod_filter.as_ref().map(|pattern| {
        RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Error: invalid --mod-filter regex \"{}\": {}", pattern, e);
                std::process::exit(1);
            })
    });

    let file_regex = cli.file_filter.as_ref().map(|pattern| {
        RegexBuilder::new(pattern)
            .case_insensitive(true)
            .build()
            .unwrap_or_else(|e| {
                eprintln!("Error: invalid --file-filter regex \"{}\": {}", pattern, e);
                std::process::exit(1);
            })
    });

    eprintln!("Loading RED4 hash database...");
    let hashes = red4lib::get_red4_hashes();

    eprintln!("Building load order...");
    let load_order = scanner::build_load_order(&archive_path);

    eprintln!(
        "Scanning {} archives for conflicts...",
        load_order.len()
    );
    let scan = scanner::scan_conflicts(&load_order, &archive_path, None);

    if cli.summary {
        eprintln!("Building summary...");
        let mut summary = scanner::build_summary(&scan, &load_order, &archive_path);

        if let Some(ref re) = mod_regex {
            summary.archives.retain(|a| re.is_match(&a.name));
        }

        eprintln!(
            "Found {} conflicting files across {} archives ({} fully overridden).",
            summary.total_conflicting_files,
            summary.total_archives,
            summary.fully_overridden_count
        );

        let output = serialize(&summary, cli.toon, cli.pretty);
        print_output(output);
        return;
    }

    eprintln!("Building report...");
    let mut report = scanner::build_report(
        &scan,
        &load_order,
        &archive_path,
        &hashes,
        cli.show_no_conflicts,
        cli.show_load_order,
    );

    // Apply mod filter — remove archives that don't match
    if let Some(ref re) = mod_regex {
        report.archives.retain(|a| re.is_match(&a.name));
    }

    // Apply file filter — remove non-matching file entries within each archive
    if let Some(ref re) = file_regex {
        for archive in &mut report.archives {
            archive.winning_files.retain(|f| re.is_match(&f.file));
            archive.losing_files.retain(|f| re.is_match(&f.file));
            if let Some(ref mut files) = archive.no_conflict_files {
                files.retain(|f| re.is_match(f));
            }
        }
        report.file_conflicts.retain(|f| re.is_match(&f.file));
    }

    eprintln!(
        "Found {} conflicting files across {} archives.",
        report.total_conflicting_files, report.total_archives
    );

    let output = serialize(&report, cli.toon, cli.pretty);
    print_output(output);

}

fn serialize<T: serde::Serialize>(data: &T, toon: bool, pretty: bool) -> Result<String, String> {
    if toon {
        serde_json::to_value(data)
            .map_err(|e| e.to_string())
            .and_then(|v| {
                let encoded = toon_format::encode_default(&v)
                    .map_err(|e| e.to_string())?;
                let validation: Result<serde_json::Value, _> =
                    toon_format::decode_default(&encoded);
                if let Err(e) = validation {
                    eprintln!(
                        "Warning: TOON round-trip validation failed: {}. \
                         Output may not parse correctly with all TOON decoders.",
                        e
                    );
                }
                Ok(encoded)
            })
    } else if pretty {
        serde_json::to_string_pretty(data).map_err(|e| e.to_string())
    } else {
        serde_json::to_string(data).map_err(|e| e.to_string())
    }
}

fn print_output(output: Result<String, String>) {
    match output {
        Ok(s) => println!("{}", s),
        Err(e) => {
            eprintln!("Failed to serialize: {}", e);
            std::process::exit(1);
        }
    }
}
