//! Inferring WanezGD-style tag keywords from the Grim Dawn `.arz` database.
//!
//! This replaces WanezGD_Tools' hand-maintained `gd-filter.json` `Tags` map.
//! For each tag we infer a `Kind` (Item or Affix) and a `Rarity`
//! (`itemClassification`); when a generic base name is shared across items of
//! different rarities, the tag's rarity is the most-frequent (mode) one.

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Seek};

use lib_gddb::arz::{Database, Record};

use crate::db;
use crate::keywords::{Kind, Rarity};

const ITEMS_PREFIX: &str = "records/items/";
const PREFIX_PATH: &str = "records/items/lootaffixes/prefix/";
const SUFFIX_PATH: &str = "records/items/lootaffixes/suffix/";
const FACTION_PREFIX: &str = "records/items/faction/";

const ITEM_TAG: &str = "itemNameTag";
const ITEM_RARITY: &str = "itemClassification";
const ITEM_CLASS: &str = "Class";
const AFFIX_TAG: &str = "lootRandomizerName";

/// `Class` prefixes for equippable gear (weapons, shields/offhands, armor,
/// jewelry) — the items that can roll a name-altering prefix/suffix. Everything
/// else (relics=ItemArtifact, components=QuestItem, augments, etc.) cannot.
const GEAR_CLASSES: [&str; 2] = ["Weapon", "Armor"];

/// The inferred keywords for a single tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TagInfo {
    pub kind: Kind,
    pub rarity: Rarity,
    /// Whether this item can be displayed with a name-altering prefix/suffix
    /// affix — the only reason to bake a color code into a base name (to stop
    /// the affix's color bleeding into it). True only for equippable, non-
    /// faction gear of rarity Common/Magical/Rare; Epic/Legendary uniques,
    /// relics, components and faction gear never roll a name affix.
    pub affixable: bool,
}

/// Runs the full inference pass over every database and returns the tag map.
pub fn infer<T: BufRead + Seek>(dbs: &mut [Database<T>]) -> HashMap<String, TagInfo> {
    // Only gear records are needed: to classify rarity and decide affixability.
    let records = db::iter_records(dbs, |id| id.starts_with(ITEMS_PREFIX));

    let mut tags: HashMap<String, TagInfo> = HashMap::new();
    // Per item tag: rarity -> #records. A handful of generic base-name tags are
    // shared across items of different rarities (e.g. a name used by both Rare
    // and Epic items), so we pick the most-frequent (mode) rarity.
    let mut rarity_counts: HashMap<String, HashMap<Rarity, u32>> = HashMap::new();
    // item tags with at least one record under records/items/faction/.
    let mut faction_tags: HashSet<String> = HashSet::new();
    // item tags with at least one equippable-gear record (can roll name affixes).
    let mut gear_tags: HashSet<String> = HashSet::new();

    for record in &records {
        let id = record.id.as_str();
        if id.starts_with(PREFIX_PATH) || id.starts_with(SUFFIX_PATH) {
            classify_affix(record, &mut tags);
        } else {
            accumulate_item(record, &mut rarity_counts, &mut faction_tags, &mut gear_tags);
        }
    }

    // Build item entries, choosing each tag's most-frequent (mode) rarity. We do
    // NOT special-case awakened/upgraded variants: they're always Epic/Legendary
    // (never colored), so including them never changes a colored result — only
    // the colorable Common/Magical/Rare tiers matter here.
    for (tag, counts) in &rarity_counts {
        let Some(rarity) = mode_rarity(counts) else {
            continue;
        };
        let affixable = gear_tags.contains(tag)
            && !faction_tags.contains(tag)
            && matches!(rarity, Rarity::Common | Rarity::Magical | Rarity::Rare);
        tags.insert(
            tag.clone(),
            TagInfo {
                kind: Kind::Item,
                rarity,
                affixable,
            },
        );
    }

    tags
}

fn classify_affix(record: &Record, tags: &mut HashMap<String, TagInfo>) {
    let Some(tag) = string_field(record, AFFIX_TAG) else {
        return;
    };
    let Some(rarity) = string_field(record, ITEM_RARITY).and_then(|s| Rarity::from_db(&s)) else {
        return;
    };
    if tag.is_empty() {
        return;
    }
    // `affixable` is about base names; an affix tag is always colored directly.
    tags.insert(
        tag,
        TagInfo {
            kind: Kind::Affix,
            rarity,
            affixable: false,
        },
    );
}

/// Accumulates an item record into the per-tag rarity tally, the faction set,
/// and the gear set. The final item entry (with the mode rarity) is built after
/// the scan.
fn accumulate_item(
    record: &Record,
    rarity_counts: &mut HashMap<String, HashMap<Rarity, u32>>,
    faction_tags: &mut HashSet<String>,
    gear_tags: &mut HashSet<String>,
) {
    let Some(tag) = string_field(record, ITEM_TAG) else {
        return;
    };
    if tag.is_empty() {
        return;
    }

    if let Some(class) = string_field(record, ITEM_CLASS) {
        if GEAR_CLASSES.iter().any(|g| class.starts_with(g)) {
            gear_tags.insert(tag.clone());
        }
    }

    let Some(rarity) = string_field(record, ITEM_RARITY).and_then(|s| Rarity::from_db(&s)) else {
        return;
    };
    *rarity_counts
        .entry(tag.clone())
        .or_default()
        .entry(rarity)
        .or_default() += 1;
    if record.id.starts_with(FACTION_PREFIX) {
        faction_tags.insert(tag);
    }
}

/// The most frequent rarity in `counts`; ties resolve to the lower tier (the
/// `Rarity` enum's `Ord` ranks low-to-high, so we compare it reversed).
fn mode_rarity(counts: &HashMap<Rarity, u32>) -> Option<Rarity> {
    counts
        .iter()
        .max_by(|a, b| a.1.cmp(b.1).then_with(|| b.0.cmp(a.0)))
        .map(|(rarity, _)| *rarity)
}

fn string_field(record: &Record, key: &str) -> Option<String> {
    record.data.get(key).and_then(|v| v.as_string())
}
