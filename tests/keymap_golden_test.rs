//! Purpose: the golden regression oracle of the keymap design (§9.1) — what
//! every key does on every screen today, pinned in `tests/keymap_golden/`
//! so that the switch to `ffm.yml` can prove its defaults equal the old
//! resolvers chord by chord.
//!
//! One TSV per context, one line per key: `chord<TAB>class<TAB>result`. Every
//! run replays the tables against the live resolvers; a difference fails, and
//! a deliberate change is regenerated with the ignored test at the bottom
//! (see `tests/keymap_golden/README.md`).

#[path = "keymap_golden_test/contexts.rs"]
mod contexts;
#[path = "keymap_golden_test/defaults.rs"]
mod defaults;
#[path = "keymap_golden_test/outcome.rs"]
mod outcome;
#[path = "keymap_golden_test/overlays.rs"]
mod overlays;
#[path = "keymap_golden_test/probes.rs"]
mod probes;
#[path = "keymap_golden_test/table.rs"]
mod table;

use funny_file_manager::action::chord::Platform;
use funny_file_manager::action::KeyChord;

use contexts::CONTEXTS;
use outcome::Class;
use probes::{probes, Probe};

const HOW_TO_REGENERATE: &str =
    "If the change is deliberate, regenerate the tables and review the diff:\n  \
    cargo test --test keymap_golden_test regenerate_the_golden_tables -- --ignored";

/// The `lenient` rows per context: chords a strict subset of whose modifiers
/// yields the same command (§2.3's leniency), which exact matching (§6.2)
/// ends at step 5. Pinned beside its reason, so that a change is a reviewed
/// diff.
const LENIENT_ROWS: [(&str, usize, &str); 15] = [
    (
        "panels",
        0,
        "exact matching (§6.2): Shift+F5 is not F5, so no chord reads a subset of its modifiers",
    ),
    (
        "tree_panel",
        53,
        "all but the Ctrl chords take any modifier; Alt+F6 is F6",
    ),
    (
        "netbox_panel",
        0,
        "exact matching (spec 6.2): no list move takes Shift",
    ),
    (
        "quick_view",
        0,
        "exact matching (§6.2) removed the lenient matches",
    ),
    (
        "viewer",
        0,
        "exact matching (§6.2) removed the lenient matches",
    ),
    (
        "editor",
        0,
        "read from the keymap, which matches exactly; 106 before the switch",
    ),
    (
        "command_line",
        0,
        "reads the keymap: exact since its switch, 103 before it",
    ),
    (
        "command_line_search",
        0,
        "reads the keymap: exact since its switch, 30 before it",
    ),
    (
        "command_line_completion",
        0,
        "reads the keymap: exact since its switch, 31 before it",
    ),
    (
        "task_list",
        91,
        "moves, Enter, Ins, Space, Esc, Ctrl+letters; F-keys guarded",
    ),
    ("task_list_search", 7, "F3 with any modifier"),
    (
        "wsl_manager",
        0,
        "read from the keymap: a key runs a command with exactly its modifiers",
    ),
    (
        "language_servers",
        0,
        "every key is matched without a modifier",
    ),
    (
        "path_editor",
        0,
        "reads the keymap: a key acts with exactly the modifiers of its binding",
    ),
    (
        "vault",
        0,
        "reads the keymap: a key with a modifier more is another key",
    ),
];

#[test]
fn every_table_replays_against_its_resolver() {
    let probes = probes();
    for context in &CONTEXTS {
        if let Err(problem) = table::replay(context, &probes) {
            panic!("{problem}\n{HOW_TO_REGENERATE}");
        }
    }
}

/// Every event of the domain gives a chord on both platforms, so no key is
/// silently left out of the tables.
#[test]
fn every_domain_event_gives_a_chord_on_both_platforms() {
    for event in probes::domain_events() {
        for platform in [Platform::Unix, Platform::Windows] {
            assert!(
                platform.chord(event).is_some(),
                "{event:?} names no chord on {platform:?}"
            );
        }
    }
}

/// A row's text and the event its result was recorded for name one chord:
/// the text parses to it and both platforms read the event as it, so the
/// keymap's tests can read the tables. A key no chord names gives none
/// either way.
#[test]
fn every_row_text_and_event_name_the_same_chord() {
    for probe in probes() {
        let parsed = probe.text.parse::<KeyChord>().ok();
        assert_eq!(parsed, probe.chord, "the text `{}`", probe.text);
        for platform in [Platform::Unix, Platform::Windows] {
            let read = platform.chord(probe.event);
            assert_eq!(read, probe.chord, "`{}` on {platform:?}", probe.text);
        }
    }
}

#[test]
fn every_row_names_a_different_key() {
    let probes = probes();
    let mut names: Vec<&str> = probes.iter().map(|probe| probe.text.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names.len(), probes.len());
}

#[test]
fn lenient_rows_per_context_are_pinned() {
    let probes = probes();
    for (name, expected, reason) in LENIENT_ROWS {
        let context = CONTEXTS
            .iter()
            .find(|context| context.name == name)
            .unwrap_or_else(|| panic!("{name} is not a context"));
        let lenient = probes
            .iter()
            .filter(|probe| context.classify(probe, &(context.run)(probe.event)) == Class::Lenient)
            .count();
        assert_eq!(lenient, expected, "{name}: {reason}");
    }
    assert_eq!(
        LENIENT_ROWS.len(),
        CONTEXTS.len(),
        "every context is pinned"
    );
}

#[test]
#[ignore = "writes tests/keymap_golden/*.tsv: run on purpose, then review the diff"]
fn regenerate_the_golden_tables() {
    let probes: Vec<Probe> = probes();
    for context in &CONTEXTS {
        table::write(context, &probes)
            .unwrap_or_else(|error| panic!("{}: {error}", table::path(context).display()));
    }
}
