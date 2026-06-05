//! The closed keyword vocabularies a tag can be labelled with: `Kind` (Type),
//! `Classification`, and `Group`. Modeled as enums rather than strings so the
//! color match rules in `color.rs` are exhaustive and typo-proof.
//!
//! Each variant's `as_str()` is the exact spelling WanezGD's gd-filter.json
//! uses; `Serialize` is hand-written to defer to it, so `as_str()` is the
//! single source of truth and the `infer`/`validate` JSON stays byte-compatible.

use serde::{Serialize, Serializer};

/// A tag's Type keyword. Only the kinds gdse infers are modeled; everything
/// else in gd-filter (Skill / Property / Style / Quality / Faction Item) is out
/// of scope (Property arrives with the damage-type pass).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    RegularItem,
    MiItem,
    Affix,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::RegularItem => "Regular Item",
            Kind::MiItem => "MI Item",
            Kind::Affix => "Affix",
        }
    }
}

/// A tag's rarity — the value of gd-filter's "Classification" keyword for items
/// and affixes. Variants are declared low-to-high so the derived `Ord` doubles
/// as the rarity rank used for mode tie-breaking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rarity {
    Broken,
    Common,
    Magical,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn as_str(self) -> &'static str {
        match self {
            Rarity::Broken => "Broken",
            Rarity::Common => "Common",
            Rarity::Magical => "Magical",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        }
    }

    /// Parses a raw `itemClassification` DB value; `None` for rarities we don't
    /// model (so the caller skips that record rather than mislabeling it).
    pub fn from_db(s: &str) -> Option<Self> {
        Some(match s {
            "Broken" => Rarity::Broken,
            "Common" => Rarity::Common,
            "Magical" => Rarity::Magical,
            "Rare" => Rarity::Rare,
            "Epic" => Rarity::Epic,
            "Legendary" => Rarity::Legendary,
            _ => return None,
        })
    }
}

macro_rules! serialize_as_str {
    ($($t:ty),+) => {$(
        impl Serialize for $t {
            fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                s.serialize_str(self.as_str())
            }
        }
    )+};
}
serialize_as_str!(Kind, Rarity);
