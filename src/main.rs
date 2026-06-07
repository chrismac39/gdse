use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod color;
mod colorize;
mod conflicts;
mod db;
mod infer;
mod keywords;
mod loot_refs;
mod mi_signal;
mod property;
mod tags;
mod validate;

/// Default location of WanezGD_Tools' ground-truth tag database.
const DEFAULT_FILTER: &str = concat!(env!("HOME"), "/WanezGD_Tools/app/data/gd-filter.json");

#[derive(Parser, Debug)]
#[command(version, about = "Grim Dawn rainbow-filter tooling (Rust/Linux)", arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Infer tag keywords from the .arz DB and print them as JSON.
    Infer {
        /// Only print tags whose key contains this substring.
        #[arg(short, long)]
        filter: Option<String>,
    },
    /// Score the inference engine against WanezGD's gd-filter.json.
    Validate {
        /// Path to gd-filter.json (defaults to ~/WanezGD_Tools/app/data/gd-filter.json).
        #[arg(short = 'g', long)]
        gd_filter: Option<PathBuf>,
    },
    /// Bake color codes into the tag text and write the rewritten .txt files.
    Colorize {
        /// Directory to write the colored .txt files into.
        #[arg(short, long, default_value = "out/text_en")]
        out: PathBuf,
    },
    /// Report rarity-ambiguous tags (awakened/upgraded variants reusing a tag).
    Conflicts,
    /// Print tag = value for every tag whose value contains NEEDLE.
    Grep { needle: String },
    /// Print, per item tag, which loot-table areas reference it (JSON).
    LootRefs,
    /// Dump candidate MI signals per tag (JSON).
    MiSignal,
    /// Find records whose any field value contains NEEDLE.
    Refs { needle: String },
}

fn main() {
    let args = Args::parse();
    let mut dbs = db::open_all();

    match args.cmd {
        Command::Infer { filter } => {
            let tags = infer::infer(&mut dbs);
            let filtered: std::collections::BTreeMap<_, _> = tags
                .iter()
                .filter(|(tag, _)| filter.as_ref().is_none_or(|f| tag.contains(f.as_str())))
                .collect();
            println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
        }
        Command::Validate { gd_filter } => {
            let tags = infer::infer(&mut dbs);
            let path = gd_filter.unwrap_or_else(|| PathBuf::from(DEFAULT_FILTER));
            validate::run(&tags, &path);
        }
        Command::Colorize { out } => colorize::run(&mut dbs, &out),
        Command::Conflicts => conflicts::run(&mut dbs),
        Command::Grep { needle } => tags::grep(&needle),
        Command::LootRefs => loot_refs::run(&mut dbs),
        Command::MiSignal => mi_signal::run(&mut dbs),
        Command::Refs { needle } => mi_signal::refs(&mut dbs, &needle),
    }
}
