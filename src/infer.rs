//! Inferring WanezGD-style tag keywords ({Type, Classification, Group}) from
//! the Grim Dawn `.arz` database.
//!
//! This replaces WanezGD_Tools' hand-maintained `gd-filter.json` `Tags` map.
//! We currently infer:
//!   - Regular Item (+ Group: Faction / Set / None), Classification from rarity
//!   - Affix (Group: Prefix / Suffix), Classification from rarity
//! MI (Monster Infrequent) detection is not yet implemented — those items are
//! reported as Regular Item for now.

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

const ITEM_TAG: &str = "itemNameTag";
const ITEM_RARITY: &str = "itemClassification";
const AFFIX_TAG: &str = "lootRandomizerName";
const SET_MEMBERS: &str = "setMembers";

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
    let records = db::iter_records(dbs, |id| id.starts_with(ITEMS_PREFIX));

    let mut tags: HashMap<String, TagInfo> = HashMap::new();
    // item record id -> its tag, so we can mark set members after the fact.
    let mut record_tag: HashMap<String, String> = HashMap::new();
    // item record ids referenced as members of some set.
    let mut set_members: HashSet<String> = HashSet::new();

    for record in &records {
        let id = record.id.as_str();
        if id.starts_with(PREFIX_PATH) || id.starts_with(SUFFIX_PATH) {
            classify_affix(record, &mut tags);
        } else if id.starts_with(LOOTSETS_PREFIX) {
            collect_set_members(record, &mut set_members);
        } else {
            classify_item(record, &mut tags, &mut record_tag);
        }
    }

    // Second pass: members of a set get Group=Set, overriding None/Faction.
    for member in &set_members {
        if let Some(tag) = record_tag.get(member) {
            if let Some(info) = tags.get_mut(tag) {
                info.group = "Set".to_string();
            }
        }
    }

    tags
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

fn classify_item(
    record: &Record,
    tags: &mut HashMap<String, TagInfo>,
    record_tag: &mut HashMap<String, String>,
) {
    let Some(tag) = string_field(record, ITEM_TAG) else {
        return;
    };
    if tag.is_empty() {
        return;
    }
    record_tag.insert(record.id.clone(), tag.clone());

    let Some(classification) = string_field(record, ITEM_RARITY) else {
        return;
    };
    let group = if record.id.starts_with(FACTION_PREFIX) {
        "Faction"
    } else {
        "None"
    };
    tags.insert(
        tag,
        TagInfo {
            kind: "Regular Item".to_string(),
            classification,
            group: group.to_string(),
        },
    );
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
