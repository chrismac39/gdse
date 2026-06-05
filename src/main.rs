use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod db;
mod infer;
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
}

fn main() {
    let args = Args::parse();
    let mut dbs = db::open_all();
    let tags = infer::infer(&mut dbs);

    match args.cmd {
        Command::Infer { filter } => {
            let filtered: std::collections::BTreeMap<_, _> = tags
                .iter()
                .filter(|(tag, _)| filter.as_ref().is_none_or(|f| tag.contains(f.as_str())))
                .collect();
            println!("{}", serde_json::to_string_pretty(&filtered).unwrap());
        }
        Command::Validate { gd_filter } => {
            let path = gd_filter.unwrap_or_else(|| PathBuf::from(DEFAULT_FILTER));
            validate::run(&tags, &path);
        }
    }
}
