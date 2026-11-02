//! Purpose: Filtering, typing and moving the selection in an unlocked vault.

use super::{unlocked, vault_file, vault_of, SIZES};
use criterion::{BenchmarkId, Criterion};
use funny_file_manager::kdbx::{Motion, Vault};
use std::hint::black_box;
use std::time::Instant;

/// Setting the whole filter, at three selectivities.
///
/// This is what happens on every keystroke in the entry list, and what a
/// pasted query costs in one go.
pub fn bench_filter(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault/filter");

    // "service" matches every entry, "ssh" a quarter, "zzz" none.
    let queries = [("all", "service"), ("some", "ssh"), ("none", "zzz")];

    for size in SIZES {
        let (_dir, path) = vault_file(size);
        let now = Instant::now();
        let mut session = unlocked(&path, now);
        let vault = vault_of(&mut session);

        for (label, query) in queries {
            let id = BenchmarkId::new(label, size);
            group.bench_function(id, |b| b.iter(|| filter_once(vault, query, now)));
        }
    }

    group.finish();
}

/// One whole-filter application, as a named function so the benchmark body
/// stays one closure deep — the project's nesting limit is four, and a
/// `for` inside a `for` inside a `bench_function` inside an `iter` is five.
fn filter_once(vault: &mut Vault, query: &str, now: Instant) -> usize {
    vault.set_filter(black_box(query), now);
    black_box(vault.matches().len())
}

/// One typed character, and one backspace.
///
/// Appending narrows an existing match set; backspace cannot, and has to
/// re-scan from the full list. They are measured apart because only one of
/// them can be made incremental, and a change that speeds up typing at the
/// cost of backspace would otherwise look free.
pub fn bench_keystroke(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault/keystroke");

    for size in SIZES {
        let (_dir, path) = vault_file(size);
        let now = Instant::now();
        let mut session = unlocked(&path, now);
        let vault = vault_of(&mut session);

        let push = BenchmarkId::new("push_char", size);
        group.bench_function(push, |b| b.iter(|| push_once(vault, now)));

        let pop = BenchmarkId::new("pop_char", size);
        group.bench_function(pop, |b| b.iter(|| pop_once(vault, now)));
    }

    group.finish();
}

/// Type `e` onto `servic`, narrowing an existing match set.
fn push_once(vault: &mut Vault, now: Instant) -> usize {
    vault.set_filter("servic", now);
    vault.push_filter_char(black_box('e'), now);
    black_box(vault.matches().len())
}

/// Backspace over the `e`, which has to widen the set again.
fn pop_once(vault: &mut Vault, now: Instant) -> usize {
    vault.set_filter("service", now);
    vault.pop_filter_char(now);
    black_box(vault.matches().len())
}

/// Moving the cursor: one row, and one page.
///
/// Cheap by construction, measured so it stays that way — the selection is an
/// index into the match list, and anything that turns it into a search over
/// the entries would show up here first.
pub fn bench_selection(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault/selection");

    for size in SIZES {
        let (_dir, path) = vault_file(size);
        let now = Instant::now();
        let mut session = unlocked(&path, now);
        let vault = vault_of(&mut session);

        let next = BenchmarkId::new("next", size);
        group.bench_function(next, |b| b.iter(|| move_once(vault, Motion::Next, now)));

        let page = BenchmarkId::new("page_down", size);
        group.bench_function(page, |b| {
            b.iter(|| move_once(vault, Motion::PageDown(20), now));
        });
    }

    group.finish();
}

/// One cursor move, returning the resulting index so it cannot be elided.
fn move_once(vault: &mut Vault, motion: Motion, now: Instant) -> usize {
    vault.move_selection(black_box(motion), now);
    black_box(vault.selected())
}
