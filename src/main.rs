use std::path::PathBuf;

use clap::Parser;

mod color;
mod colorize;
mod db;
mod infer;
mod keywords;
mod property;

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
}

fn main() {
    let args = Args::parse();
    let lang = args.language.to_lowercase();
    let mut dbs = db::open_all();
    let out = args
        .out
        .unwrap_or_else(|| db::install_path().join("settings").join(format!("text_{lang}")));
    colorize::run(&mut dbs, &out, &lang);
}
