//! Inferring WanezGD-style tag keywords from the Grim Dawn `.arz` database.
//!
//! This replaces WanezGD_Tools' hand-maintained `gd-filter.json` `Tags` map.
//! For each tag we infer a `Kind` (Type) and a `Rarity` (Classification):
//!   - Regular Item, rarity from `itemClassification` (mode across its records)
//!   - Affix, rarity from the affix record's `itemClassification`
//!   - MI (Monster Infrequent): a Rare, non-faction item that scales to the
//!     level cap and whose drop is rolled by a `tdyn_*` dynamic affix table.
//!     Only Rare MIs are flagged — epic/legendary "MIs" aren't mechanically
//!     distinguishable in the DB (see mi-domain-model memory).

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Seek};

use lib_gddb::arz::{Database, DatabaseValue, Record};
use serde::Serialize;

use crate::db;
use crate::keywords::{Kind, Rarity};

const ITEMS_PREFIX: &str = "records/items/";
const PREFIX_PATH: &str = "records/items/lootaffixes/prefix/";
const SUFFIX_PATH: &str = "records/items/lootaffixes/suffix/";
const FACTION_PREFIX: &str = "records/items/faction/";
const LOOTTABLES_PREFIX: &str = "records/items/loottables/";

const ITEM_TAG: &str = "itemNameTag";
const ITEM_RARITY: &str = "itemClassification";
const ITEM_LEVEL: &str = "itemLevel";
const ITEM_CLASS: &str = "Class";
const AFFIX_TAG: &str = "lootRandomizerName";

/// `Class` prefixes for equippable gear (weapons, shields/offhands, armor,
/// jewelry) — the items that can roll a name-altering prefix/suffix. Everything
/// else (relics=ItemArtifact, components=QuestItem, augments, etc.) cannot.
const GEAR_CLASSES: [&str; 2] = ["Weapon", "Armor"];

/// A true MI scales to the endgame level cap; one-off quest/unique rares cap
/// well below it (~70). Requiring a level-cap variant removes those FPs without
/// dropping any real MI. (Current cap; bump if a future expansion raises it.)
const MI_MIN_MAX_LEVEL: u32 = 94;

/// The inferred keywords for a single tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct TagInfo {
    #[serde(rename = "Type")]
    pub kind: Kind,
    #[serde(rename = "Classification")]
    pub rarity: Rarity,
    /// Whether this item can be displayed with a name-altering prefix/suffix
    /// affix — the only reason to bake a color code into a base name (to stop
    /// the affix's color bleeding into it). True only for equippable, non-
    /// faction gear of rarity Common/Magical/Rare; Epic/Legendary uniques,
    /// relics, components and faction gear never roll a name affix. Not part of
    /// the gd-filter schema, so skipped when serializing.
    #[serde(skip)]
    pub affixable: bool,
}

/// Runs the full inference pass over every database and returns the tag map.
pub fn infer<T: BufRead + Seek>(dbs: &mut [Database<T>]) -> HashMap<String, TagInfo> {
    // Only item records are needed: gear (to classify) and loot tables (to
    // detect MIs via the `tdyn_*` dynamic affix tables that roll their drops).
    let records = db::iter_records(dbs, |id| id.starts_with(ITEMS_PREFIX));

    let mut tags: HashMap<String, TagInfo> = HashMap::new();
    // item record id -> its tag, so we can resolve tdyn drops to tags.
    let mut record_tag: HashMap<String, String> = HashMap::new();
    // item record ids named by a `tdyn_*` dynamic affix table (resolved later,
    // since a table may be scanned before the items it references).
    let mut tdyn_refs: Vec<String> = Vec::new();
    // Per item tag: rarity -> #records, over all records and over base records
    // (excluding awakened/upgraded variants). Used to pick the canonical rarity.
    let mut rarity_counts: HashMap<String, HashMap<Rarity, u32>> = HashMap::new();
    let mut base_rarity_counts: HashMap<String, HashMap<Rarity, u32>> = HashMap::new();
    // item tags with at least one record under records/items/faction/.
    let mut faction_tags: HashSet<String> = HashSet::new();
    // item tags with at least one equippable-gear record (can roll name affixes).
    let mut gear_tags: HashSet<String> = HashSet::new();
    // item tag -> max itemLevel across its records (MI scaling signal).
    let mut max_levels: HashMap<String, u32> = HashMap::new();

    for record in &records {
        let id = record.id.as_str();
        if id.starts_with(LOOTTABLES_PREFIX) {
            collect_tdyn_refs(record, &mut tdyn_refs);
        } else if id.starts_with(PREFIX_PATH) || id.starts_with(SUFFIX_PATH) {
            classify_affix(record, &mut tags);
        } else {
            accumulate_item(
                record,
                &mut record_tag,
                &mut rarity_counts,
                &mut base_rarity_counts,
                &mut faction_tags,
                &mut gear_tags,
                &mut max_levels,
            );
        }
    }

    // Build Regular Item entries, choosing each tag's most-frequent (mode)
    // rarity. Awakened/upgraded variants are excluded so e.g. a Legendary
    // awakened copy doesn't override an Epic base (and the lone Rare variant of
    // a mostly-Common item doesn't win).
    for (tag, all_counts) in &rarity_counts {
        let counts = base_rarity_counts
            .get(tag)
            .filter(|c| !c.is_empty())
            .unwrap_or(all_counts);
        let Some(rarity) = mode_rarity(counts) else {
            continue;
        };
        let affixable = gear_tags.contains(tag)
            && !faction_tags.contains(tag)
            && matches!(rarity, Rarity::Common | Rarity::Magical | Rarity::Rare);
        tags.insert(tag.clone(), TagInfo { kind: Kind::RegularItem, rarity, affixable });
    }

    // A tag is a Monster Infrequent if a `tdyn_*` affix table rolls its drop.
    // That signal alone is noisy (tdyn tables also list low-level commons), so
    // we gate it: MIs are Rare, non-faction, and scale to the level cap. With
    // those gates this is 99.6% recall / 100% precision vs gd-filter.
    let via_tdyn: HashSet<&String> = tdyn_refs.iter().filter_map(|id| record_tag.get(id)).collect();
    for (tag, info) in tags.iter_mut() {
        let scales_to_cap = max_levels.get(tag).copied().unwrap_or(0) >= MI_MIN_MAX_LEVEL;
        if info.kind == Kind::RegularItem
            && info.rarity == Rarity::Rare
            && !faction_tags.contains(tag)
            && scales_to_cap
            && via_tdyn.contains(tag)
        {
            info.kind = Kind::MiItem;
        }
    }

    tags
}

/// Collects the item records a `tdyn_*` (dynamic affix) loot table names via its
/// `lootName*` fields. Non-tdyn tables are ignored.
fn collect_tdyn_refs(record: &Record, out: &mut Vec<String>) {
    if !record.id.contains("tdyn") {
        return;
    }
    for (key, value) in record.data.iter() {
        if key.starts_with("lootName") {
            if let DatabaseValue::String(s) = value {
                out.push(s.clone());
            }
        }
    }
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
    tags.insert(tag, TagInfo { kind: Kind::Affix, rarity, affixable: false });
}

/// Accumulates an item record into the per-tag rarity tallies, the faction set,
/// and the record->tag map. The final Regular Item entry (with the mode rarity)
/// is built after the scan; MI status is decided in a later post-pass.
fn accumulate_item(
    record: &Record,
    record_tag: &mut HashMap<String, String>,
    rarity_counts: &mut HashMap<String, HashMap<Rarity, u32>>,
    base_rarity_counts: &mut HashMap<String, HashMap<Rarity, u32>>,
    faction_tags: &mut HashSet<String>,
    gear_tags: &mut HashSet<String>,
    max_levels: &mut HashMap<String, u32>,
) {
    let Some(tag) = string_field(record, ITEM_TAG) else {
        return;
    };
    if tag.is_empty() {
        return;
    }
    record_tag.insert(record.id.clone(), tag.clone());

    if let Some(class) = string_field(record, ITEM_CLASS) {
        if GEAR_CLASSES.iter().any(|g| class.starts_with(g)) {
            gear_tags.insert(tag.clone());
        }
    }

    let level = record.data.get(ITEM_LEVEL).and_then(|v| v.as_int()).unwrap_or(0);
    let entry = max_levels.entry(tag.clone()).or_insert(0);
    *entry = (*entry).max(level);

    let Some(rarity) = string_field(record, ITEM_RARITY).and_then(|s| Rarity::from_db(&s)) else {
        return;
    };
    *rarity_counts.entry(tag.clone()).or_default().entry(rarity).or_default() += 1;
    let is_variant = record.id.contains("/awakened/") || record.id.contains("/upgraded/");
    if !is_variant {
        *base_rarity_counts.entry(tag.clone()).or_default().entry(rarity).or_default() += 1;
    }
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
