//! Opening the Grim Dawn `.arz` databases and iterating records.
//!
//! Adapted from the consumption patterns in `~/gddb/src/util.rs`.

use std::fs::{File, canonicalize};
use std::io::{BufRead, BufReader, Seek};
use std::path::PathBuf;

use lib_gddb::arz::{Database, RawRecord, Record};

/// Relative paths to the base game + expansion databases, in load order.
const DBS: [&str; 4] = [
    "database/database.arz",
    "gdx1/database/GDX1.arz",
    "gdx2/database/GDX2.arz",
    "gdx3/database/GDX3.arz",
];

/// Resolves `GRIM_DAWN_INSTALL_PATH` to a canonical install directory.
pub fn install_path() -> PathBuf {
    let raw = std::env::var_os("GRIM_DAWN_INSTALL_PATH").unwrap_or_else(|| {
        eprintln!("Please set GRIM_DAWN_INSTALL_PATH");
        std::process::exit(1);
    });
    canonicalize(PathBuf::from(&raw)).unwrap_or_else(|e| {
        eprintln!("Could not resolve GRIM_DAWN_INSTALL_PATH={:?}: {e}", raw);
        std::process::exit(1);
    })
}

/// Opens every available database. Missing expansion DBs are skipped.
pub fn open_all() -> Vec<Database<BufReader<File>>> {
    let base = install_path();
    let dbs: Vec<_> = DBS
        .iter()
        .filter_map(|rel| Database::open(base.join(rel)).ok())
        .collect();
    if dbs.is_empty() {
        eprintln!(
            "Could not read any database files under {}",
            base.display()
        );
        std::process::exit(1);
    }
    dbs
}

/// Resolves every record whose id satisfies `keep`, across all databases.
///
/// Filtering on the (cheap) record id before resolving avoids decompressing
/// records we don't care about.
pub fn iter_records<T: BufRead + Seek>(
    dbs: &mut [Database<T>],
    keep: impl Fn(&str) -> bool + Copy,
) -> Vec<Record> {
    let mut out = Vec::new();
    for db in dbs.iter_mut() {
        let raws: Vec<RawRecord> = db
            .iter_records()
            .expect("iter_records")
            .collect::<Result<_, _>>()
            .expect("collect raw records");
        for raw in raws {
            let Ok(id) = db.record_id(&raw) else { continue };
            if keep(&id) {
                if let Ok(record) = db.resolve(raw) {
                    out.push(record);
                }
            }
        }
    }
    out
}
