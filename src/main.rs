use std::path::PathBuf;
use std::{fs::OpenOptions, io::Write};

use clap::Parser;
use time::macros::format_description;
use time::{OffsetDateTime, UtcOffset};

mod color;
mod colorize;
mod db;
mod infer;
mod keywords;
mod palette;
mod property;

use property::DamageColors;

/// Reads Grim Dawn resource files and recolors text tags to make damage
/// types and affix rarity legible at a glance.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Args {
    /// Language to recolor
    #[arg(short, long, default_value = "en")]
    language: String,
    /// Output path
    /// [default: $GRIM_DAWN_INSTALL_PATH/settings/text_<language>/].
    #[arg(short, long)]
    out: Option<PathBuf>,
    /// Paints Pierce red, like rainbow filter, not pink.
    #[arg(long)]
    rainbow_filter_damage_colors: bool,
}

fn main() {
    let args = Args::parse();
    let lang = args.language.to_lowercase();
    let db_hash = db::database_hash();
    let steam_build_id = db::steam_build_id().unwrap_or_else(|| "unknown".to_string());
    let patch_versions = colorize::detect_patch_versions(&lang);
    let mut dbs = db::open_all();
    let out = args.out.unwrap_or_else(|| {
        db::install_path()
            .join("settings")
            .join(format!("text_{lang}"))
    });
    let damage_colors = if args.rainbow_filter_damage_colors {
        DamageColors::RainbowFilter
    } else {
        DamageColors::Default
    };

    let hash_file = out.join("gdse-db-hash.txt");
    match std::fs::read_to_string(&hash_file) {
        Ok(previous_log) => {
            if latest_logged_hash(&previous_log).is_some_and(|h| h == db_hash) {
                println!("Database hash unchanged since last run; output should be up to date.");
            } else {
                println!("Database hash changed since last run; rewriting output.");
            }
        }
        Err(_) => {
            println!("No previous database hash found; generating output.");
        }
    }

    if patch_versions.is_empty() {
        println!("No patch version label detected in text archives.");
    } else {
        println!(
            "Detected patch version label(s): {}",
            patch_versions.join(", ")
        );
    }

    colorize::run(&mut dbs, &out, &lang, damage_colors);

    let now = local_timestamp_minute();
    let patch_versions_field = if patch_versions.is_empty() {
        "unknown".to_string()
    } else {
        patch_versions.join(",")
    };
    let mut f = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&hash_file)
        .unwrap_or_else(|e| {
            eprintln!("Could not open {}: {e}", hash_file.display());
            std::process::exit(1);
        });
    if let Err(e) = writeln!(
        f,
        "{now} hash={db_hash} steam_build_id={steam_build_id} patch_versions={patch_versions_field}"
    ) {
        eprintln!("Could not write {}: {e}", hash_file.display());
        std::process::exit(1);
    }
}

fn latest_logged_hash(log: &str) -> Option<&str> {
    for line in log.lines().rev() {
        if let Some(token) = line
            .split_whitespace()
            .find(|token| token.starts_with("hash="))
        {
            return token.strip_prefix("hash=");
        }

        // Backward compatibility with old lines: "YYYY-MM-DD HH:MM <hash>"
        let tokens: Vec<_> = line.split_whitespace().collect();
        if tokens.len() >= 3 {
            return Some(tokens[2]);
        }
    }
    None
}

fn local_timestamp_minute() -> String {
    // Prefer local machine time for user-facing history output.
    let now = UtcOffset::current_local_offset()
        .map(|offset| OffsetDateTime::now_utc().to_offset(offset))
        .unwrap_or_else(|_| OffsetDateTime::now_utc());
    let fmt = format_description!("[year]-[month]-[day] [hour]:[minute]");
    now.format(&fmt)
        .unwrap_or_else(|_| "unknown-time".to_string())
}
