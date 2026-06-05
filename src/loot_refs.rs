//! Analysis: test whether "has a dedicated loot table" predicts Monster
//! Infrequent status. For each item tag we compute the smallest number of
//! distinct item tags in any loot table that references it — MIs are expected
//! to have a dedicated (single-item) affix table, regulars to share big tables.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Seek};

use lib_gddb::arz::{Database, DatabaseValue};
use serde::Serialize;

use crate::db;

const ITEMS_PREFIX: &str = "records/items/";
const LOOTTABLES_PREFIX: &str = "records/items/loottables/";
const ITEM_TAG: &str = "itemNameTag";

#[derive(Serialize)]
struct TagStat {
    tag: String,
    /// Smallest distinct-tag count among loot tables referencing this tag.
    min_table_tags: usize,
    /// How many distinct loot tables reference this tag.
    table_count: usize,
    /// Whether some referencing table also names affix tables (rolls affixes).
    has_affix_table: bool,
}

pub fn run<T: BufRead + Seek>(dbs: &mut [Database<T>]) {
    let items = db::iter_records(dbs, |id| id.starts_with(ITEMS_PREFIX));
    let mut record_tag: HashMap<String, String> = HashMap::new();
    for r in &items {
        if let Some(tag) = r.data.get(ITEM_TAG).and_then(|v| v.as_string()) {
            record_tag.insert(r.id.clone(), tag);
        }
    }

    // For each loot table: the set of distinct item tags it references, and
    // whether it specifies any affix (prefix/suffix) table.
    // Then fold into per-tag stats.
    let mut min_tags: HashMap<String, usize> = HashMap::new();
    let mut table_count: HashMap<String, usize> = HashMap::new();
    let mut has_affix: HashMap<String, bool> = HashMap::new();

    for r in &items {
        if !r.id.starts_with(LOOTTABLES_PREFIX) {
            continue;
        }
        let mut tags_in_table: HashSet<&str> = HashSet::new();
        let mut affix_table = false;
        for (key, value) in r.data.iter() {
            if let DatabaseValue::String(s) = value {
                if key.starts_with("lootName") {
                    if let Some(tag) = record_tag.get(s) {
                        tags_in_table.insert(tag);
                    }
                } else if (key.contains("refixTableName") || key.contains("uffixTableName"))
                    && !s.is_empty()
                {
                    affix_table = true;
                }
            }
        }
        let size = tags_in_table.len();
        if size == 0 {
            continue;
        }
        for tag in tags_in_table {
            let e = min_tags.entry(tag.to_string()).or_insert(usize::MAX);
            *e = (*e).min(size);
            *table_count.entry(tag.to_string()).or_default() += 1;
            let a = has_affix.entry(tag.to_string()).or_insert(false);
            *a = *a || affix_table;
        }
    }

    let mut out: Vec<TagStat> = min_tags
        .into_iter()
        .map(|(tag, min_table_tags)| TagStat {
            table_count: table_count[&tag],
            has_affix_table: has_affix[&tag],
            tag,
            min_table_tags,
        })
        .collect();
    out.sort_by(|a, b| a.tag.cmp(&b.tag));
    println!("{}", serde_json::to_string(&out).unwrap());
}
