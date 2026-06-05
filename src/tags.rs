//! Loading the localized tag text from the `.arc` resource bundles, and a
//! grep helper for inspecting how Grim Dawn natively uses `{^X}` color codes.

use std::collections::HashMap;

use lib_gddb::arc::Archive;
use lib_gddb::tags;

use crate::db::install_path;

/// Relative paths to the English text bundles, base game + expansions.
const TEXT_ARCS: [&str; 4] = [
    "resources/Text_EN.arc",
    "gdx1/resources/Text_EN.arc",
    "gdx2/resources/Text_EN.arc",
    "gdx3/resources/Text_EN.arc",
];

/// Loads and merges every tag from every available text bundle.
/// Later (expansion) bundles override earlier ones on key collisions.
pub fn load_all() -> HashMap<String, String> {
    let base = install_path();
    let mut all = HashMap::new();
    for rel in TEXT_ARCS {
        let Ok(mut arc) = Archive::open(base.join(rel)) else {
            continue;
        };
        let Ok(records) = arc.iter_records() else {
            continue;
        };
        for record in records.flatten() {
            // Tag text lives in the `tags*.txt` records; skip everything else.
            if !record.id.contains("tag") {
                continue;
            }
            if let Ok(parsed) = tags::parse(&record.data) {
                all.extend(parsed);
            }
        }
    }
    all
}

/// Prints `tag = value` for every tag whose value contains `needle`.
pub fn grep(needle: &str) {
    let all = load_all();
    let mut hits: Vec<(&String, &String)> = all
        .iter()
        .filter(|(_, v)| v.contains(needle))
        .collect();
    hits.sort();
    for (tag, value) in &hits {
        println!("{tag} = {value:?}");
    }
    eprintln!("\n{} tags contain {:?}", hits.len(), needle);
}
