//! Purpose: what the `KeePass` vault costs — the one slow wait, and the keystroke paths.
//!
//! The vault has exactly two kinds of cost, and conflating them hides both:
//!
//! - **Unlocking** is a deliberate wait. Its time is the key-derivation
//!   function, which is *supposed* to be slow — that is the security property.
//!   What is worth watching is that decrypting and cataloguing the entries
//!   afterwards stays small next to the KDF, so a large vault does not quietly
//!   turn a 1-second unlock into a 5-second one.
//! - **Everything after unlocking** runs on the event loop, between a key
//!   press and the next frame. Typing one character into the filter re-scans
//!   the whole entry list; moving the cursor re-derives the visible window. A
//!   vault of a few hundred entries is ordinary, a few thousand is not rare,
//!   and neither may cost enough to be felt.
//!
//! The filter is measured at three selectivities on purpose. A query matching
//! almost everything and one matching almost nothing do the same amount of
//! scanning but very different amounts of collecting, and a regression in one
//! is invisible in the other.

use std::hint::black_box;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::kdbx::{KdbxSession, Motion, Vault};
use funny_file_manager::keepass::{create_database, EntryDetails};
use tempfile::TempDir;

/// Vault sizes: a personal vault, a team vault, and one large enough that an
/// accidental quadratic in the filter would be unmistakable.
const SIZES: [usize; 3] = [100, 1_000, 5_000];

const PASSWORD: &str = "bench-master-password";

/// Entries spread over a few groups, with titles that share prefixes — the
/// shape a filter actually meets. Uniform titles would let a prefix test
/// short-circuit in a way real data does not.
fn entries(count: usize) -> Vec<EntryDetails> {
    (0..count)
        .zip(["api", "web", "db", "ssh"].into_iter().cycle())
        .map(|(index, kind)| EntryDetails {
            title: format!("service-{index:04}-{kind}"),
            username: format!("user{}@example.com", index % 50),
            url: format!("https://host-{:03}.example.com", index % 200),
            notes: String::new(),
            password: format!("secret-{index:06}"),
        })
        .collect()
}

/// A saved vault. Kept alive by its `TempDir`, so the caller must hold both.
#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn vault_file(count: usize) -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("vault benchmark dir");
    let path = dir.path().join("bench.kdbx");
    create_database(&path, PASSWORD, None, &entries(count)).expect("write benchmark vault");
    (dir, path)
}

fn unlocked(path: &Path, now: Instant) -> KdbxSession {
    let mut session = KdbxSession::new(path.to_path_buf());
    for ch in PASSWORD.chars() {
        session.type_char(ch, now);
    }
    assert!(session.unlock(now), "benchmark vault must unlock");
    session
}

#[expect(
    clippy::expect_used,
    reason = "criterion has no error channel: a failed fixture or measured call must stop the run"
)]
fn vault_of(session: &mut KdbxSession) -> &mut Vault {
    session.vault_mut().expect("vault is unlocked")
}

/// Opening a vault: KDF, decrypt, and catalogue the entries.
///
/// Sampled sparsely because a real KDF is measured in hundreds of milliseconds
/// by design; the question here is not "is it fast" but "does it grow with the
/// entry count". If the three sizes land within noise of each other, the KDF
/// dominates and the catalogue is free — which is what we want to keep true.
fn bench_unlock(c: &mut Criterion) {
    let mut group = c.benchmark_group("vault/unlock");
    group.sample_size(10);
    group.measurement_time(Duration::from_secs(10));

    for size in SIZES {
        let (_dir, path) = vault_file(size);
        let now = Instant::now();
        group.bench_function(BenchmarkId::from_parameter(size), |b| {
            b.iter(|| black_box(unlocked(&path, now)));
        });
    }

    group.finish();
}

/// Setting the whole filter, at three selectivities.
///
/// This is what happens on every keystroke in the entry list, and what a
/// pasted query costs in one go.
fn bench_filter(c: &mut Criterion) {
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
fn bench_keystroke(c: &mut Criterion) {
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
fn bench_selection(c: &mut Criterion) {
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

criterion_group!(
    benches,
    bench_unlock,
    bench_filter,
    bench_keystroke,
    bench_selection
);
criterion_main!(benches);
