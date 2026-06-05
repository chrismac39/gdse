//! Analysis of rarity-ambiguous tags: tags whose item records disagree on
//! `itemClassification`, in particular the awakened/upgraded gdx3 variants that
//! reuse a base item's tag at a higher rarity.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, Seek};

use lib_gddb::arz::Database;

use crate::db;

const ITEMS_PREFIX: &str = "records/items/";
const ITEM_TAG: &str = "itemNameTag";
const ITEM_RARITY: &str = "itemClassification";

#[derive(Default)]
struct TagRarities {
    /// rarity -> count, for "normal" records (not awakened/upgraded).
    base: BTreeMap<String, usize>,
    awakened: BTreeMap<String, usize>,
    upgraded: BTreeMap<String, usize>,
}

fn mode(counts: &BTreeMap<String, usize>) -> Option<&str> {
    counts.iter().max_by_key(|(_, n)| **n).map(|(k, _)| k.as_str())
}

pub fn run<T: BufRead + Seek>(dbs: &mut [Database<T>]) {
    let records = db::iter_records(dbs, |id| id.starts_with(ITEMS_PREFIX));

    let mut by_tag: HashMap<String, TagRarities> = HashMap::new();
    for r in &records {
        let Some(tag) = r.data.get(ITEM_TAG).and_then(|v| v.as_string()) else {
            continue;
        };
        let Some(rarity) = r.data.get(ITEM_RARITY).and_then(|v| v.as_string()) else {
            continue;
        };
        let entry = by_tag.entry(tag).or_default();
        let bucket = if r.id.contains("/awakened/") {
            &mut entry.awakened
        } else if r.id.contains("/upgraded/") {
            &mut entry.upgraded
        } else {
            &mut entry.base
        };
        *bucket.entry(rarity).or_default() += 1;
    }

    let total = by_tag.len();
    let mut with_awakened = 0usize;
    // (base canonical rarity, awakened canonical rarity) -> count
    let mut pair_counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    // base-only tags that are still rarity-ambiguous (multiple base rarities)
    let mut base_ambiguous = 0usize;
    let mut awakened_non_legendary: Vec<String> = Vec::new();
    let mut examples: Vec<(String, String, String)> = Vec::new();

    for (tag, tr) in &by_tag {
        if tr.base.len() > 1 {
            base_ambiguous += 1;
        }
        if tr.awakened.is_empty() {
            continue;
        }
        with_awakened += 1;
        let base = mode(&tr.base).unwrap_or("(none)").to_string();
        let awk = mode(&tr.awakened).unwrap_or("(none)").to_string();
        if tr.awakened.keys().any(|r| r != "Legendary") {
            awakened_non_legendary.push(tag.clone());
        }
        if base != awk && examples.len() < 12 {
            examples.push((tag.clone(), base.clone(), awk.clone()));
        }
        *pair_counts.entry((base, awk)).or_default() += 1;
    }

    println!("=== Rarity-ambiguity / awakened analysis ===");
    println!("distinct item tags:                 {total}");
    println!("tags with an awakened variant:      {with_awakened}");
    println!("base-only tags w/ mixed rarities:   {base_ambiguous}");
    println!();
    println!("base rarity -> awakened rarity (count):");
    for ((base, awk), n) in &pair_counts {
        let flag = if base == awk { "" } else { "  <- conflict" };
        println!("  {base:<11} -> {awk:<11} {n}{flag}");
    }
    println!();
    println!(
        "awakened records NOT Legendary: {} tags",
        awakened_non_legendary.len()
    );
    for t in awakened_non_legendary.iter().take(10) {
        println!("  {t}");
    }
    if !examples.is_empty() {
        println!();
        println!("sample conflicts (tag: base -> awakened):");
        for (tag, base, awk) in &examples {
            println!("  {tag}: {base} -> {awk}");
        }
    }
}
