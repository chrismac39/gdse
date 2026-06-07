//! Vendored damage-type / Property tag color map.
//!
//! Which tags are damage / attribute / misc Property labels, and which element
//! each belongs to, is curated from gd-filter (`data/properties_en.tsv`, one
//! `tag<TAB>element` per line). That classification is the only piece not
//! derivable from the game data. The label TEXT itself is read live from the
//! game's `tags_ui.txt` by the colorizer, so this stays gdx3-correct with no
//! vendored text snapshot — tags absent from the player's game simply never
//! match and are left alone.

const DATA: &str = include_str!("../data/properties_en.tsv");

/// element -> color letter, ported from Full Rainbow's `Core.Property.*` rules.
/// Each element's over-time variant shares the base element's color (Burn=Fire,
/// Frostburn=Cold, Electrocute=Lightning, Poison=Acid, Decay=Vitality,
/// Trauma=Physical, Bleeding=Pierce).
fn element_color(element: &str) -> Option<char> {
    Some(match element {
        "Physical" | "Trauma" => 'k',
        "Pierce" | "Bleeding" => 'r',
        "Fire" | "Burn" => 'o',
        "Cold" | "Frostburn" => 'c',
        "Lightning" | "Electrocute" => 'z',
        "Acid" | "Poison" => 'l',
        "Vitality" | "Decay" => 'm',
        "Aether" => 'a',
        "Chaos" => 'p',
        "Elemental" => 'y',
        "Attribute" => 'f',
        "Misc" => 'x',
        _ => return None,
    })
}

/// `(tag, color)` for every Property tag with a known element color.
pub fn colors() -> Vec<(&'static str, char)> {
    DATA.lines()
        .filter_map(|line| {
            let (tag, element) = line.split_once('\t')?;
            Some((tag, element_color(element)?))
        })
        .collect()
}
