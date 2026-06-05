//! Analysis: compute candidate MI signals per item tag and dump them as JSON
//! for offline correlation against gd-filter.
//!
//! Signals:
//!   - dedicated:  the item is the sole tag in some loot table (dedicated affix table)
//!   - via_table:  reachable from a loot table that a creature references (a DROP)
//!   - via_equip:  directly referenced by a creature record (WORN equipment; noisy)

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Seek};

use lib_gddb::arz::{Database, DatabaseValue};
use serde::Serialize;

use crate::db;

const ITEM_TAG: &str = "itemNameTag";
const LOOTTABLES_PREFIX: &str = "records/items/loottables/";
const CREATURES_PREFIX: &str = "records/creatures/";

#[derive(Serialize)]
struct TagSignals {
    tag: String,
    dedicated: bool,
    via_table: bool,
    via_equip: bool,
    via_tdyn: bool,
    max_level: u32,
}

pub fn run<T: BufRead + Seek>(dbs: &mut [Database<T>]) {
    // Single pass over the entire database.
    let all = db::iter_records(dbs, |_| true);

    // item record id -> tag
    let mut record_tag: HashMap<String, String> = HashMap::new();
    // item tag -> max itemLevel across its records
    let mut max_level: HashMap<String, u32> = HashMap::new();
    // loot table id -> record ids it references (items and/or sub-tables)
    let mut table_refs: HashMap<String, Vec<String>> = HashMap::new();
    // loot tables referenced by some creature (drop chains)
    let mut creature_tables: HashSet<String> = HashSet::new();
    // item record ids referenced directly by some creature (worn equipment)
    let mut creature_items: HashSet<String> = HashSet::new();
    // item tags referenced by a `tdyn_*` (dynamic affix) loot table
    let mut via_tdyn_tags: HashSet<String> = HashSet::new();

    for r in &all {
        if r.id.starts_with("records/items/") {
            if let Some(tag) = r.data.get(ITEM_TAG).and_then(|v| v.as_string()) {
                record_tag.insert(r.id.clone(), tag.clone());
                let lvl = r.data.get("itemLevel").and_then(|v| v.as_int()).unwrap_or(0);
                let e = max_level.entry(tag).or_insert(0);
                *e = (*e).max(lvl);
            }
        }
    }

    for r in &all {
        let is_table = r.id.starts_with(LOOTTABLES_PREFIX);
        let is_creature = r.id.starts_with(CREATURES_PREFIX);
        for (key, value) in r.data.iter() {
            let DatabaseValue::String(s) = value else { continue };
            if is_table && key.starts_with("lootName") {
                table_refs.entry(r.id.clone()).or_default().push(s.clone());
                if r.id.contains("tdyn") {
                    if let Some(tag) = record_tag.get(s) {
                        via_tdyn_tags.insert(tag.clone());
                    }
                }
            }
            if is_creature {
                if s.starts_with(LOOTTABLES_PREFIX) {
                    creature_tables.insert(s.clone());
                } else if record_tag.contains_key(s) {
                    creature_items.insert(s.clone());
                }
            }
        }
    }

    // dedicated: sole distinct tag in some table
    let mut dedicated: HashSet<String> = HashSet::new();
    for refs in table_refs.values() {
        let tags: HashSet<&str> = refs
            .iter()
            .filter_map(|id| record_tag.get(id).map(|s| s.as_str()))
            .collect();
        if tags.len() == 1 {
            dedicated.insert(tags.into_iter().next().unwrap().to_string());
        }
    }

    // via_table: BFS from creature-referenced tables, following table->table edges,
    // collecting reachable item records.
    let mut via_table_tags: HashSet<String> = HashSet::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut queue: Vec<String> = creature_tables.iter().cloned().collect();
    while let Some(id) = queue.pop() {
        if !seen.insert(id.clone()) {
            continue;
        }
        if let Some(refs) = table_refs.get(&id) {
            for r in refs {
                if let Some(tag) = record_tag.get(r) {
                    via_table_tags.insert(tag.clone());
                } else if r.starts_with(LOOTTABLES_PREFIX) {
                    queue.push(r.clone());
                }
            }
        }
    }

    let via_equip_tags: HashSet<String> = creature_items
        .iter()
        .filter_map(|id| record_tag.get(id).cloned())
        .collect();

    let all_tags: HashSet<String> = record_tag.values().cloned().collect();
    let mut out: Vec<TagSignals> = all_tags
        .into_iter()
        .map(|tag| TagSignals {
            dedicated: dedicated.contains(&tag),
            via_table: via_table_tags.contains(&tag),
            via_equip: via_equip_tags.contains(&tag),
            via_tdyn: via_tdyn_tags.contains(&tag),
            max_level: max_level.get(&tag).copied().unwrap_or(0),
            tag,
        })
        .collect();
    out.sort_by(|a, b| a.tag.cmp(&b.tag));
    println!("{}", serde_json::to_string(&out).unwrap());
}

/// Reverse reference: print records (id, field=value) whose any string field
/// value contains `needle`.
pub fn refs<T: BufRead + Seek>(dbs: &mut [Database<T>], needle: &str) {
    let all = db::iter_records(dbs, |_| true);
    for r in &all {
        for (key, value) in r.data.iter() {
            if let DatabaseValue::String(s) = value {
                if s.contains(needle) {
                    println!("{}  {}={}", r.id, key, s);
                }
            }
        }
    }
}
