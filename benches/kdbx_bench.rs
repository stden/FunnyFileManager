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

use std::time::{Duration, Instant};

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

#[path = "kdbx_bench/browsing.rs"]
mod browsing;
#[path = "kdbx_bench/vaults.rs"]
mod vaults;
use browsing::{bench_filter, bench_keystroke, bench_selection};
use vaults::{unlocked, vault_file, vault_of, SIZES};

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

criterion_group!(
    benches,
    bench_unlock,
    bench_filter,
    bench_keystroke,
    bench_selection
);
criterion_main!(benches);
