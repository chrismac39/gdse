//! The color model: a port of WanezGD's FilterGroup + Library concept, trimmed
//! to gdse's locked v1 scope (see scope-decision memory).
//!
//! A `Rule` is a match over a tag's inferred Kind + Rarity plus the color
//! letter to assign. `LIBRARY` is an ordered list of rules; the first one a tag
//! matches wins, so it encodes the color hierarchy. An empty `rarities` slice is
//! a wildcard; `kinds` is always required (a tag with no matching Kind never
//! matches, mirroring WanezGD's required Type).
//!
//! Colors are the single-letter `{^X}` codes from WanezGD's gd-colorcodes.json.
//! We keep only what the locked scope colors: item & affix rarity and the
//! distinct Rare-MI palette. Faction / Set / Skill / Quality / Style get no
//! special cue — items take their rarity color regardless, matching Full
//! Rainbow. Damage-type Property rules arrive in a later pass (curated
//! tag→element map).

use crate::infer::TagInfo;
use crate::keywords::{Kind, Rarity};

struct Rule {
    kinds: &'static [Kind],
    rarities: &'static [Rarity],
    color: char,
}

impl Rule {
    fn matches(&self, info: &TagInfo) -> bool {
        self.kinds.contains(&info.kind)
            && (self.rarities.is_empty() || self.rarities.contains(&info.rarity))
    }
}

/// The active preset, ordered first-match-wins. Seeded from Full Rainbow's
/// `fullRainbow` library, trimmed to gdse's scope. Only rarities that can carry
/// a name-altering affix appear: Common=w, Magical=y, Rare=g. Epic/Legendary are
/// absent on purpose — those names never take a prefix/suffix, so we leave them
/// to the engine's native rarity color (see `color_for`'s `affixable` gate).
///
/// The one MI distinction we make is Rare: a Rare MI reads olive instead of the
/// regular-Rare green, so the Rare-MI rule comes first.
const ITEMS: &[Kind] = &[Kind::RegularItem, Kind::Affix, Kind::MiItem];
const LIBRARY: &[Rule] = &[
    use_rarity(&[Kind::MiItem], Rarity::Rare, 'l'),
    use_rarity(ITEMS, Rarity::Common, 'w'),
    use_rarity(ITEMS, Rarity::Magical, 'y'),
    use_rarity(ITEMS, Rarity::Rare, 'g'),
];

/// A rule matching `kinds` of a single rarity.
const fn use_rarity(kinds: &'static [Kind], rarity: Rarity, color: char) -> Rule {
    Rule { kinds, rarities: rarity_slice(rarity), color }
}

/// `const`-context helper: a one-element `Rarity` slice. (Can't take a reference
/// to a temporary in a `const`, so we map each rarity to a static.)
const fn rarity_slice(rarity: Rarity) -> &'static [Rarity] {
    match rarity {
        Rarity::Broken => &[Rarity::Broken],
        Rarity::Common => &[Rarity::Common],
        Rarity::Magical => &[Rarity::Magical],
        Rarity::Rare => &[Rarity::Rare],
        Rarity::Epic => &[Rarity::Epic],
        Rarity::Legendary => &[Rarity::Legendary],
    }
}

/// The color letter to bake into `tag`'s value, or `None` if the tag is left
/// untouched. Base item names are only colored when they can take a name-
/// altering affix (otherwise the engine's native rarity color is fine and there
/// is no bleed to guard against); affix tags are always colored directly.
pub fn color_for(info: &TagInfo) -> Option<char> {
    if matches!(info.kind, Kind::RegularItem | Kind::MiItem) && !info.affixable {
        return None;
    }
    LIBRARY.iter().find(|r| r.matches(info)).map(|r| r.color)
}
