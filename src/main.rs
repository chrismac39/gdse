use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod color;
mod colorize;
mod db;
mod infer;
mod keywords;
mod property;

#[derive(Parser, Debug)]
#[command(version, about = "Grim Dawn rainbow-filter tooling (Rust/Linux)", arg_required_else_help = true)]
struct Args {
    #[command(subcommand)]
    cmd: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Bake color codes into the tag text and write the rewritten .txt files.
    Colorize {
        /// Directory to write the colored .txt files into.
        #[arg(short, long, default_value = "out/text_en")]
        out: PathBuf,
        /// Color Monster Infrequents by plain rarity (no distinct olive cue).
        #[arg(long)]
        plain_mi: bool,
    },
}

fn main() {
    let args = Args::parse();
    let mut dbs = db::open_all();

    match args.cmd {
        Command::Colorize { out, plain_mi } => colorize::run(&mut dbs, &out, !plain_mi),
    }
}
