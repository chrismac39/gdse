//! Damage-type Property tag coloring, derived from the tag NAME alone.
//!
//! Grim Dawn's stat-label tags follow a rigid naming convention: a structural
//! prefix (`Damage`/`Defense`/`Retaliation`/`tagConversion`/`tagDamageBase`)
//! plus the damage element's base token (`Fire`, `Cold`, `Aether`, …). The
//! over-time and acid variants reuse the base token in the name — `Burn` labels
//! are `DamageDuration*Fire*`, `Acid` is `*Poison*`, `Trauma` is `*Physical*`,
//! `Frostburn` is `*Cold*`, `Decay`/`Vitality` is `*Life*` — so this small base
//! vocabulary covers every damage-type label. Tag names are English keys
//! regardless of the localization, so the rule is language-independent and needs
//! no vendored data. Non-damage stat labels (attributes, OA/DA, speeds, …) carry
//! no element token and so are left untouched.

use crate::palette::{
    AQUA, COBALT, CYAN, FUSHIA, KHAKI, MAROON, OLIVE, ORANGE, PURPLE, RED, YELLOW,
};

/// Structural prefixes that mark a damage / resistance / retaliation /
/// conversion stat label. A property tag always starts with one of these.
const PREFIXES: [&str; 5] = [
    "Damage",
    "Defense",
    "Retaliation",
    "tagConversion",
    "tagDamageBase",
];

/// Which damage-type palette to paint with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DamageColors {
    /// gdse's default: Pierce is Fushia, so it reads apart from Bleeding's Red.
    Default,
    /// Full Rainbow's familiar palette, where Pierce and Bleeding are both Red.
    RainbowFilter,
}

/// Base Grim Dawn element tokens embedded in stat-label tag names, mapped to the
/// Full Rainbow `Core.Property.*` element color. Over-time variants share the
/// base element's color, so e.g. `Fire` covers Burn and `Poison` covers Acid.
///
/// Pierce is the one departure from Full Rainbow, which paints it and Bleeding
/// both Red and so makes the two indistinguishable. Pierce takes Fushia — the
/// only bright palette slot with no other use, and far enough from Purple
/// (Chaos), its nearest neighbour, to read clearly. Teal was the other free slot
/// but sits between Cyan (Cold) and Aqua (Aether), so it would have traded one
/// collision for another. [`DamageColors::RainbowFilter`] restores the familiar
/// Red.
const TOKENS: [(&str, char); 12] = [
    ("Physical", KHAKI),
    ("Pierce", FUSHIA),
    ("Bleeding", RED),
    ("Fire", ORANGE),
    ("Cold", CYAN),
    ("Lightning", COBALT),
    ("Poison", OLIVE),
    ("Vitality", MAROON),
    ("Life", MAROON),
    ("Aether", AQUA),
    ("Chaos", PURPLE),
    ("Elemental", YELLOW),
];

/// The color letter for a damage-type Property tag, or `None` if `tag` isn't one.
pub fn color_for(tag: &str, damage_colors: DamageColors) -> Option<char> {
    if !PREFIXES.iter().any(|p| tag.starts_with(p)) {
        return None;
    }
    // `*Reduction*` tags are "Reduced target's X Damage/Resistance" enemy-debuff
    // labels — they name an element but describe a debuff you inflict, not the
    // item's own damage/resist, so coloring them by element would mislead.
    if tag.contains("Reduction") {
        return None;
    }
    // `DamageModifierPierceRatio[R]` ("Increases Armor Piercing by X%") is a
    // deprecated stat the game no longer uses (Greg confirmed 2026-06). The live
    // Armor Piercing label is `DamageBasePierceRatio`, which is kept.
    if tag.starts_with("DamageModifierPierceRatio") {
        return None;
    }
    // Resist-duration labels ("Reduction in Burn Duration", "Wound Duration
    // Reduction") are descriptive phrases, not plain element labels. Coloring
    // the whole sentence reads badly, and coloring only the element word would
    // need English-only value parsing — so skip them entirely (Greg, 2026-06).
    if tag.starts_with("Defense") && tag.contains("Duration") {
        return None;
    }
    // The `Life`/`Vitality` token doubles as the health pool: Life Leech and
    // %-Health stats name it but aren't the Vitality damage type. Leave those
    // uncolored (they're sustain/utility, like the dropped Misc group).
    if tag.contains("Leech")
        || tag.contains("Leach")
        || (tag.contains("Percent") && tag.contains("Life"))
    {
        return None;
    }
    let color = TOKENS
        .iter()
        .find(|(tok, _)| tag.contains(tok))
        .map(|(_, color)| *color)?;
    // Opt back in to Full Rainbow's Red Pierce. Fushia is the only entry gdse
    // departs on, so folding it back to Red is the whole of the flag.
    if damage_colors == DamageColors::RainbowFilter && color == FUSHIA {
        return Some(RED);
    }
    Some(color)
}
