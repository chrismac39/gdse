//! Scores the inference engine against WanezGD_Tools' hand-maintained
//! `gd-filter.json`, treating its `Tags` map as ground truth.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::Deserialize;

use crate::infer::TagInfo;

/// The subset of `gd-filter.json` we care about.
#[derive(Debug, Deserialize)]
struct GdFilter {
    #[serde(rename = "Tags")]
    tags: HashMap<String, GdTag>,
}

#[derive(Debug, Deserialize)]
struct GdTag {
    #[serde(rename = "Type")]
    kind: Option<String>,
    #[serde(rename = "Classification")]
    classification: Option<String>,
}

/// Types our engine currently attempts to infer; everything else in
/// gd-filter (Skill/Property/Style/Quality) is out of scope for now.
const IN_SCOPE: [&str; 3] = ["Regular Item", "MI Item", "Affix"];

pub fn run(inferred: &HashMap<String, TagInfo>, filter_path: &Path) {
    let raw = match std::fs::read_to_string(filter_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Could not read {}: {e}", filter_path.display());
            std::process::exit(1);
        }
    };
    let truth: GdFilter = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("Could not parse {}: {e}", filter_path.display());
            std::process::exit(1);
        }
    };

    let total_truth = truth.tags.len();
    let in_scope: Vec<(&String, &GdTag)> = truth
        .tags
        .iter()
        .filter(|(_, t)| t.kind.as_deref().is_some_and(|k| IN_SCOPE.contains(&k)))
        .collect();

    // Coverage: of the in-scope ground-truth tags, how many did we produce?
    let mut covered = 0usize;
    let mut missing_by_type: BTreeMap<&str, usize> = BTreeMap::new();

    // Agreement among covered tags.
    let mut agree_type = 0usize;
    let mut agree_class = 0usize;
    let mut agree_all = 0usize;

    // Confusion: ground-truth Type -> our Type, with counts.
    let mut type_confusion: BTreeMap<(&str, &str), usize> = BTreeMap::new();
    // A few concrete mismatches to eyeball.
    let mut class_mismatches: Vec<(String, String, String)> = Vec::new();

    for (tag, gt) in &in_scope {
        let gt_type = gt.kind.as_deref().unwrap_or("");
        let Some(got) = inferred.get(*tag) else {
            *missing_by_type.entry(gt_type).or_default() += 1;
            continue;
        };
        covered += 1;

        let t_ok = got.kind.as_str() == gt_type;
        let c_ok = gt
            .classification
            .as_deref()
            .is_none_or(|c| c == got.rarity.as_str());

        if t_ok {
            agree_type += 1;
        } else {
            *type_confusion
                .entry((gt_type, got.kind.as_str()))
                .or_default() += 1;
        }
        if c_ok {
            agree_class += 1;
        } else if class_mismatches.len() < 15 {
            class_mismatches.push((
                (*tag).clone(),
                gt.classification.clone().unwrap_or_default(),
                got.rarity.as_str().to_string(),
            ));
        }
        if t_ok && c_ok {
            agree_all += 1;
        }
    }

    let extra = inferred
        .keys()
        .filter(|tag| !truth.tags.contains_key(*tag))
        .count();

    let pct = |n: usize, d: usize| if d == 0 { 0.0 } else { 100.0 * n as f64 / d as f64 };

    println!("=== Inference vs gd-filter.json ===");
    println!("ground-truth tags:        {total_truth}");
    println!("  in-scope (item/affix):  {}", in_scope.len());
    println!("inferred tags (total):    {}", inferred.len());
    println!("  not in ground truth:    {extra}");
    println!();
    println!(
        "coverage (in-scope found): {covered}/{} ({:.1}%)",
        in_scope.len(),
        pct(covered, in_scope.len())
    );
    if !missing_by_type.is_empty() {
        println!("  missing by truth Type:");
        for (k, n) in &missing_by_type {
            println!("    {k:<14} {n}");
        }
    }
    println!();
    println!("agreement among {covered} covered tags:");
    println!("  Type:            {:.1}%", pct(agree_type, covered));
    println!("  Classification:  {:.1}%", pct(agree_class, covered));
    println!("  both:            {:.1}%", pct(agree_all, covered));

    if !type_confusion.is_empty() {
        println!();
        println!("Type confusion (truth -> inferred):");
        for ((gt, got), n) in &type_confusion {
            println!("  {gt:<14} -> {got:<14} {n}");
        }
    }
    if !class_mismatches.is_empty() {
        println!();
        println!("sample Classification mismatches (tag: truth -> inferred):");
        for (tag, gt, got) in &class_mismatches {
            println!("  {tag}: {gt} -> {got}");
        }
    }
}
