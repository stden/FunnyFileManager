//! Purpose: Each module uses only its own layer or a layer below it.
//!
//! `module_layers/map.rs` says which layer each module is in. References come
//! from the syn-based resolver in `architecture`, so comments, strings, use
//! groups, renames and crate-root aliases are resolved before layers compare.

#[path = "module_layers/fixtures.rs"]
mod fixtures;
#[path = "module_layers/map.rs"]
mod map;

use super::architecture::{forbidden_dependencies, SourceFile, Violation};
use super::tui_sources;
use map::{CYCLIC_MODULES, IGNORED_MODULES, LAYERS, UPWARD_DEBT};
use std::collections::{BTreeMap, BTreeSet};

/// A layer table and the upward uses it tolerates.
pub struct LayerMap<'a> {
    pub layers: &'a [(&'a str, &'a [&'a str])],
    pub ignored: &'a [&'a str],
    pub debt: &'a [(&'a str, &'a str)],
}

/// The real crate's table.
pub const CRATE: LayerMap<'static> = LayerMap {
    layers: &LAYERS,
    ignored: IGNORED_MODULES,
    debt: UPWARD_DEBT,
};

/// One resolved reference from a top-level module to another.
struct Edge {
    source: String,
    target: String,
    place: String,
    path: String,
}

fn top_module(file: &SourceFile) -> Option<&str> {
    file.module.first().map(String::as_str)
}

fn layer_index(map: &LayerMap<'_>, module: &str) -> Option<usize> {
    map.layers
        .iter()
        .position(|(_, modules)| modules.contains(&module))
}

/// Every crate-internal path a checked production file names, as resolved.
fn edges(sources: &[SourceFile], map: &LayerMap<'_>) -> Vec<Edge> {
    let checked = |file: &SourceFile| {
        !file.is_test() && top_module(file).is_some_and(|top| !map.ignored.contains(&top))
    };
    let tops: BTreeMap<&str, &str> = sources
        .iter()
        .filter_map(|file| Some((file.relative.as_str(), top_module(file)?)))
        .collect();
    forbidden_dependencies(sources, checked, &["crate"])
        .into_iter()
        .filter_map(|reference: Violation| {
            let source = (*tops.get(reference.file.as_str())?).to_owned();
            let target = reference.path.split("::").nth(1)?.to_owned();
            (source != target).then(|| Edge {
                source,
                target,
                place: format!("{}:{}", reference.file, reference.line),
                path: reference.path,
            })
        })
        .collect()
}

/// Modules with no layer, layer entries with no module, and both checked
/// against the real modules in `sources`.
fn membership(sources: &[SourceFile], map: &LayerMap<'_>) -> Vec<String> {
    let present: BTreeSet<&str> = sources.iter().filter_map(top_module).collect();
    // How many layers list each module: more than one is a module in two layers.
    let mut listed: BTreeMap<&str, usize> = BTreeMap::new();
    for module in map
        .layers
        .iter()
        .flat_map(|(_, modules)| modules.iter().copied())
    {
        *listed.entry(module).or_default() += 1;
    }
    let unlayered = present
        .iter()
        .filter(|module| !listed.contains_key(*module) && !map.ignored.contains(*module))
        .map(|module| format!("module `{module}` is in no layer"));
    let twice = listed
        .iter()
        .filter(|(_, count)| **count > 1)
        .map(|(module, _)| format!("module `{module}` is in two layers"));
    let missing = listed
        .keys()
        .filter(|module| !present.contains(*module))
        .map(|module| format!("layer entry `{module}` names no module in src/"));
    unlayered.chain(twice).chain(missing).collect()
}

fn is_upward(map: &LayerMap<'_>, source: &str, target: &str) -> bool {
    matches!(
        (layer_index(map, source), layer_index(map, target)),
        (Some(from), Some(to)) if to > from
    )
}

fn layer_name<'a>(map: &LayerMap<'a>, module: &str) -> &'a str {
    layer_index(map, module)
        .and_then(|index| map.layers.get(index))
        .map_or("no layer", |(name, _)| *name)
}

/// Upward uses outside the debt, once per pair of modules, and debt entries that are gone.
pub fn layer_violations(sources: &[SourceFile], map: &LayerMap<'_>) -> Vec<String> {
    let mut messages = membership(sources, map);
    // Per pair: where it first appears, what it resolves to, and how often it is named.
    let mut upward: BTreeMap<(String, String), (String, String, usize)> = BTreeMap::new();
    for edge in edges(sources, map) {
        if is_upward(map, &edge.source, &edge.target) {
            upward
                .entry((edge.source, edge.target))
                .or_insert((edge.place, edge.path, 0))
                .2 += 1;
        }
    }
    for ((source, target), (place, path, count)) in &upward {
        if !map.debt.contains(&(source.as_str(), target.as_str())) {
            messages.push(format!(
                "{place}: `{path}` ({source} in {} uses {target} in {}, a higher layer; named {count} time(s))",
                layer_name(map, source),
                layer_name(map, target),
            ));
        }
    }
    let gone = map
        .debt
        .iter()
        .filter(|(source, target)| {
            !upward.contains_key(&((*source).to_owned(), (*target).to_owned()))
        })
        .map(|(source, target)| format!("debt `{source}` -> `{target}` is gone: remove it"));
    messages.extend(gone);
    messages
}

/// Modules on a loop with modules of their own layer.
pub fn cyclic_modules(sources: &[SourceFile], map: &LayerMap<'_>) -> BTreeSet<String> {
    let mut next: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for edge in edges(sources, map) {
        let same_layer = matches!(
            (layer_index(map, &edge.source), layer_index(map, &edge.target)),
            (Some(from), Some(to)) if from == to
        );
        if same_layer {
            next.entry(edge.source).or_default().insert(edge.target);
        }
    }
    next.keys()
        .filter(|module| reaches_itself(&next, module))
        .cloned()
        .collect()
}

/// Whether `start` can be reached again by following `next` from it.
fn reaches_itself(next: &BTreeMap<String, BTreeSet<String>>, start: &str) -> bool {
    let mut stack: Vec<&str> = next
        .get(start)
        .into_iter()
        .flatten()
        .map(String::as_str)
        .collect();
    let mut seen = BTreeSet::new();
    while let Some(module) = stack.pop() {
        if module == start {
            return true;
        }
        if seen.insert(module) {
            stack.extend(next.get(module).into_iter().flatten().map(String::as_str));
        }
    }
    false
}

#[test]
fn modules_use_only_their_own_layer_or_one_below() {
    assert_eq!(
        layer_violations(&tui_sources(), &CRATE),
        Vec::<String>::new()
    );
}

#[test]
fn modules_on_a_loop_inside_one_layer_are_the_known_ones() {
    let expected: BTreeSet<String> = CYCLIC_MODULES
        .iter()
        .map(|module| (*module).to_owned())
        .collect();
    assert_eq!(cyclic_modules(&tui_sources(), &CRATE), expected);
}
