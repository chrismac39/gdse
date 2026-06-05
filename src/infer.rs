//! Inferring WanezGD-style tag keywords ({Type, Classification, Group}) from
//! the Grim Dawn `.arz` database.
//!
//! This replaces WanezGD_Tools' hand-maintained `gd-filter.json` `Tags` map.
//! We infer:
//!   - Regular Item (+ Group: Faction / Set / None), Classification from rarity
//!   - Affix (Group: Prefix / Suffix), Classification from rarity
//!   - MI (Monster Infrequent): a Rare, non-faction item that scales to the
//!     level cap and whose drop is rolled by a `tdyn_*` dynamic affix table.
//!     Only Rare MIs are flagged — epic/legendary "MIs" aren't mechanically
//!     distinguishable in the DB (see mi-domain-model memory).

use std::collections::{HashMap, HashSet};
use std::io::{BufRead, Seek};

use lib_gddb::arz::{Database, DatabaseValue, Record};
use serde::Serialize;

use crate::db;

const ITEMS_PREFIX: &str = "records/items/";
const PREFIX_PATH: &str = "records/items/lootaffixes/prefix/";
const SUFFIX_PATH: &str = "records/items/lootaffixes/suffix/";
const FACTION_PREFIX: &str = "records/items/faction/";
const LOOTSETS_PREFIX: &str = "records/items/lootsets/";
const LOOTTABLES_PREFIX: &str = "records/items/loottables/";

const ITEM_TAG: &str = "itemNameTag";
const ITEM_RARITY: &str = "itemClassification";
const ITEM_LEVEL: &str = "itemLevel";
const AFFIX_TAG: &str = "lootRandomizerName";
const SET_MEMBERS: &str = "setMembers";
const RARE: &str = "Rare";

/// A true MI scales to the endgame level cap; one-off quest/unique rares cap
/// well below it (~70). Requiring a level-cap variant removes those FPs without
/// dropping any real MI. (Current cap; bump if a future expansion raises it.)
const MI_MIN_MAX_LEVEL: u32 = 94;


/// The inferred keywords for a single tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TagInfo {
    #[serde(rename = "Type")]
    pub kind: String,
    #[serde(rename = "Classification")]
    pub classification: String,
    #[serde(rename = "Group")]
    pub group: String,
}

/// Runs the full inference pass over every database and returns the tag map.
pub fn infer<T: BufRead + Seek>(dbs: &mut [Database<T>]) -> HashMap<String, TagInfo> {
    // Only item records are needed: gear (to classify) and loot tables (to
    // detect MIs via the `tdyn_*` dynamic affix tables that roll their drops).
    let records = db::iter_records(dbs, |id| id.starts_with(ITEMS_PREFIX));

    let mut tags: HashMap<String, TagInfo> = HashMap::new();
    // item record id -> its tag, so we can resolve set members and tdyn drops.
    let mut record_tag: HashMap<String, String> = HashMap::new();
    // item record ids referenced as members of some set.
    let mut set_members: HashSet<String> = HashSet::new();
    // item record ids named by a `tdyn_*` dynamic affix table (resolved later,
    // since a table may be scanned before the items it references).
    let mut tdyn_refs: Vec<String> = Vec::new();
    // Per item tag: rarity -> #records, over all records and over base records
    // (excluding awakened/upgraded variants). Used to pick the canonical rarity.
    let mut rarity_counts: HashMap<String, HashMap<String, u32>> = HashMap::new();
    let mut base_rarity_counts: HashMap<String, HashMap<String, u32>> = HashMap::new();
    // item tags with at least one record under records/items/faction/.
    let mut faction_tags: HashSet<String> = HashSet::new();
    // item tag -> max itemLevel across its records (MI scaling signal).
    let mut max_levels: HashMap<String, u32> = HashMap::new();

    for record in &records {
        let id = record.id.as_str();
        if id.starts_with(LOOTTABLES_PREFIX) {
            collect_tdyn_refs(record, &mut tdyn_refs);
        } else if id.starts_with(PREFIX_PATH) || id.starts_with(SUFFIX_PATH) {
            classify_affix(record, &mut tags);
        } else if id.starts_with(LOOTSETS_PREFIX) {
            collect_set_members(record, &mut set_members);
        } else {
            accumulate_item(
                record,
                &mut record_tag,
                &mut rarity_counts,
                &mut base_rarity_counts,
                &mut faction_tags,
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
        let Some(classification) = mode_rarity(counts) else {
            continue;
        };
        let group = if faction_tags.contains(tag) {
            "Faction"
        } else {
            "None"
        };
        tags.insert(
            tag.clone(),
            TagInfo {
                kind: "Regular Item".to_string(),
                classification,
                group: group.to_string(),
            },
        );
    }

    // Members of a set get Group=Set, overriding None/Faction.
    for member in &set_members {
        if let Some(tag) = record_tag.get(member) {
            if let Some(info) = tags.get_mut(tag) {
                info.group = "Set".to_string();
            }
        }
    }

    // A tag is a Monster Infrequent if a `tdyn_*` affix table rolls its drop.
    // That signal alone is noisy (tdyn tables also list low-level commons), so
    // we gate it: MIs are Rare, non-faction, and scale to the level cap. With
    // those gates this is 99.6% recall / 100% precision vs gd-filter.
    let via_tdyn: HashSet<&String> = tdyn_refs.iter().filter_map(|id| record_tag.get(id)).collect();
    for (tag, info) in tags.iter_mut() {
        let scales_to_cap = max_levels.get(tag).copied().unwrap_or(0) >= MI_MIN_MAX_LEVEL;
        if info.kind == "Regular Item"
            && info.classification == RARE
            && info.group != "Faction"
            && scales_to_cap
            && via_tdyn.contains(tag)
        {
            info.kind = "MI Item".to_string();
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
    if tag.is_empty() {
        return;
    }
    let group = if record.id.starts_with(PREFIX_PATH) {
        "Prefix"
    } else {
        "Suffix"
    };
    tags.insert(
        tag,
        TagInfo {
            kind: "Affix".to_string(),
            classification: string_field(record, ITEM_RARITY).unwrap_or_default(),
            group: group.to_string(),
        },
    );
}

/// Accumulates an item record into the per-tag rarity tallies, the faction set,
/// and the record->tag map. The final Regular Item entry (with the mode rarity)
/// is built after the scan; MI status is decided in a later post-pass.
fn accumulate_item(
    record: &Record,
    record_tag: &mut HashMap<String, String>,
    rarity_counts: &mut HashMap<String, HashMap<String, u32>>,
    base_rarity_counts: &mut HashMap<String, HashMap<String, u32>>,
    faction_tags: &mut HashSet<String>,
    max_levels: &mut HashMap<String, u32>,
) {
    let Some(tag) = string_field(record, ITEM_TAG) else {
        return;
    };
    if tag.is_empty() {
        return;
    }
    record_tag.insert(record.id.clone(), tag.clone());

    let level = record.data.get(ITEM_LEVEL).and_then(|v| v.as_int()).unwrap_or(0);
    let entry = max_levels.entry(tag.clone()).or_insert(0);
    *entry = (*entry).max(level);

    let Some(rarity) = string_field(record, ITEM_RARITY) else {
        return;
    };
    *rarity_counts
        .entry(tag.clone())
        .or_default()
        .entry(rarity.clone())
        .or_default() += 1;
    let is_variant = record.id.contains("/awakened/") || record.id.contains("/upgraded/");
    if !is_variant {
        *base_rarity_counts
            .entry(tag.clone())
            .or_default()
            .entry(rarity)
            .or_default() += 1;
    }
    if record.id.starts_with(FACTION_PREFIX) {
        faction_tags.insert(tag);
    }
}

/// Rarity tiers low-to-high, for tie-breaking the mode toward the lower tier.
fn rarity_rank(rarity: &str) -> u8 {
    match rarity {
        "Broken" => 0,
        "Common" => 1,
        "Magical" => 2,
        "Rare" => 3,
        "Epic" => 4,
        "Legendary" => 5,
        _ => 6,
    }
}

/// The most frequent rarity in `counts`; ties resolve to the lower tier.
fn mode_rarity(counts: &HashMap<String, u32>) -> Option<String> {
    counts
        .iter()
        .max_by(|a, b| {
            a.1.cmp(b.1)
                .then_with(|| rarity_rank(b.0).cmp(&rarity_rank(a.0)))
        })
        .map(|(rarity, _)| rarity.clone())
}

fn collect_set_members(record: &Record, set_members: &mut HashSet<String>) {
    match record.data.get(SET_MEMBERS) {
        Some(DatabaseValue::String(s)) => {
            set_members.insert(s.clone());
        }
        Some(DatabaseValue::Strings(ss)) => {
            set_members.extend(ss.iter().filter(|s| !s.is_empty()).cloned());
        }
        _ => {}
    }
}

fn string_field(record: &Record, key: &str) -> Option<String> {
    record.data.get(key).and_then(|v| v.as_string())
}
