//! The colorizer: bakes `{^X}` color codes into the localization tag text and
//! writes the rewritten `.txt` files for Grim Dawn to load.
//!
//! Source text is read from the game's pristine `Text_EN.arc` bundles (never the
//! already-colored files in settings/text_en, which would double-color). Each
//! tag we have a color for has its value rewritten by `apply_color` (a port of
//! WanezGD's placement cascade), and any file that changed is written whole —
//! comments, Desc lines and untouched tags preserved — to the output directory.

use std::collections::HashMap;
use std::io::{BufRead, Seek};
use std::path::Path;

use lib_gddb::arc::Archive;
use lib_gddb::arz::Database;

use crate::color;
use crate::db::install_path;
use crate::infer;
use crate::property;

/// English text bundles, base game + expansions, in load order.
const TEXT_ARCS: [&str; 4] = [
    "resources/Text_EN.arc",
    "gdx1/resources/Text_EN.arc",
    "gdx2/resources/Text_EN.arc",
    "gdx3/resources/Text_EN.arc",
];

/// Infers tag colors, rewrites the text bundles, and writes the changed `.txt`
/// files under `out_dir`. Returns nothing; prints a summary.
pub fn run<T: BufRead + Seek>(dbs: &mut [Database<T>], out_dir: &Path) {
    let colors = color_map(dbs);

    if let Err(e) = std::fs::create_dir_all(out_dir) {
        eprintln!("Could not create {}: {e}", out_dir.display());
        std::process::exit(1);
    }

    let base = install_path();
    let mut files_written = 0usize;
    let mut tags_colored = 0usize;

    for rel in TEXT_ARCS {
        let Ok(mut arc) = Archive::open(base.join(rel)) else {
            continue;
        };
        let Ok(records) = arc.iter_records() else {
            continue;
        };
        for record in records.flatten() {
            // Tag text lives in the `tags*.txt` records; skip everything else.
            if !record.id.contains("tag") || !record.id.ends_with(".txt") {
                continue;
            }
            let text = String::from_utf8_lossy(&record.data);
            let (rewritten, colored) = recolor_file(&text, &colors);
            if colored == 0 {
                continue;
            }
            let dest = out_dir.join(&record.id);
            if let Err(e) = std::fs::write(&dest, rewritten) {
                eprintln!("Could not write {}: {e}", dest.display());
                std::process::exit(1);
            }
            files_written += 1;
            tags_colored += colored;
        }
    }

    println!(
        "Colored {tags_colored} tags across {files_written} files -> {}",
        out_dir.display()
    );
}

/// Builds the tag -> color-letter map for every tag the active preset colors:
/// DB-inferred item / affix / MI tags, plus the curated Property (damage-type)
/// tags. Property values are read live from the game text, so they're colored
/// in place by `recolor_file` like any other tag.
fn color_map<T: BufRead + Seek>(dbs: &mut [Database<T>]) -> HashMap<String, char> {
    let mut map: HashMap<String, char> = infer::infer(dbs)
        .into_iter()
        .filter_map(|(tag, info)| color::color_for(&info).map(|c| (tag, c)))
        .collect();
    map.extend(property::colors().into_iter().map(|(tag, c)| (tag.to_string(), c)));
    map
}

/// Rewrites one `.txt` file's content, recoloring each line whose tag is in
/// `colors`. Returns the new content and how many tag values actually changed.
/// Comments, blank lines, and untouched tags are preserved verbatim.
fn recolor_file(text: &str, colors: &HashMap<String, char>) -> (String, usize) {
    let mut out = String::with_capacity(text.len());
    let mut colored = 0usize;
    for segment in text.split_inclusive('\n') {
        let (line, eol) = split_eol(segment);
        if let Some((tag, value)) = line.split_once('=') {
            if let Some(&color) = colors.get(tag) {
                let mut new_value = apply_color(value, color);
                // Conversion labels carry no placeholder, so the color would
                // bleed to the line's end; close it with `{^E}` (WanezGD's rule).
                if tag.contains("Conversion") {
                    new_value.push_str("{^E}");
                }
                if new_value != value {
                    colored += 1;
                }
                out.push_str(tag);
                out.push('=');
                out.push_str(&new_value);
                out.push_str(eol);
                continue;
            }
        }
        out.push_str(segment);
    }
    (out, colored)
}

/// Splits a trailing `\n` or `\r\n` off a line segment, returning (body, eol).
fn split_eol(segment: &str) -> (&str, &str) {
    if let Some(body) = segment.strip_suffix("\r\n") {
        (body, "\r\n")
    } else if let Some(body) = segment.strip_suffix('\n') {
        (body, "\n")
    } else {
        (segment, "")
    }
}

/// Bakes color code `color` into a tag value, porting WanezGD's placement
/// cascade (SRainbowFilter.js `ApplyColorInSourceData`). The cascade handles
/// values that already contain codes or structural prefixes; the common case
/// (a plain name) just gets the code prepended.
fn apply_color(value: &str, color: char) -> String {
    let cc = format!("{{^{}}}", color.to_ascii_uppercase());

    // `{^E}`/`{^S}` are placeholders the source uses to mark where the name's
    // color should go: replace the placeholder, then restore it at the end so
    // any trailing text returns to its original (brown/value) color.
    if value.contains("{^E}") {
        return format!("{}{{^E}}", value.replacen("{^E}", &cc, 1));
    }
    if value.contains("{^S}") {
        return format!("{}{{^S}}", value.replacen("{^S}", &cc, 1));
    }
    // An existing inline code: overwrite every code with ours.
    if has_color_code(value) {
        return replace_color_codes(value, &cc);
    }
    // A `[header]` prefix (optionally `$`-quoted): inject after the bracket.
    if value.starts_with('[') || value.starts_with("$[") {
        return insert_after_brackets(value, &cc);
    }
    // A leading `$` (no-translate marker): inject right after it.
    if let Some(rest) = value.strip_prefix('$') {
        return format!("${cc}{rest}");
    }
    // A `|n` formatting prefix: inject after the `|n`.
    if value.starts_with('|') {
        return insert_after_bar_digit(value, &cc);
    }
    // Plain name: prepend, provided there's actually a letter to color.
    if value.chars().any(|c| c.is_ascii_alphabetic()) {
        return format!("{cc}{value}");
    }
    value.to_string()
}

/// Whether a `{^X}` (X = a single letter) inline code appears anywhere in `s`.
fn has_color_code(s: &str) -> bool {
    let b = s.as_bytes();
    (0..b.len()).any(|i| is_color_code_at(b, i))
}

fn is_color_code_at(b: &[u8], i: usize) -> bool {
    b.get(i) == Some(&b'{')
        && b.get(i + 1) == Some(&b'^')
        && b.get(i + 2).is_some_and(u8::is_ascii_alphabetic)
        && b.get(i + 3) == Some(&b'}')
}

/// Replaces every `{^X}` inline code in `s` with `cc`. Slices are taken at the
/// code's ASCII boundaries, so non-ASCII text in between is preserved intact.
fn replace_color_codes(s: &str, cc: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let mut last = 0;
    while i < b.len() {
        if is_color_code_at(b, i) {
            out.push_str(&s[last..i]);
            out.push_str(cc);
            i += 4;
            last = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[last..]);
    out
}

/// Inserts `cc` after each `[letters]` header segment.
fn insert_after_brackets(s: &str, cc: &str) -> String {
    let mut out = String::with_capacity(s.len() + cc.len());
    let mut chars = s.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        out.push(c);
        if c == ']' && out[..out.len() - 1].ends_with(|c: char| c.is_ascii_alphabetic()) {
            // Only after a `[...]` that held letters; cheap check: the char
            // before `]` was a letter.
            if let Some(open) = out.rfind('[') {
                if out[open + 1..out.len() - 1].chars().all(|c| c.is_ascii_alphabetic()) {
                    out.push_str(cc);
                }
            }
        }
    }
    out
}

/// Inserts `cc` after each `|n` (pipe + digit) formatting marker.
fn insert_after_bar_digit(s: &str, cc: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len() + cc.len());
    let mut i = 0;
    let mut last = 0;
    while i < b.len() {
        if b[i] == b'|' && b.get(i + 1).is_some_and(u8::is_ascii_digit) {
            out.push_str(&s[last..i + 2]);
            out.push_str(cc);
            i += 2;
            last = i;
        } else {
            i += 1;
        }
    }
    out.push_str(&s[last..]);
    out
}
