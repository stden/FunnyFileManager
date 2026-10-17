//! Purpose: what the long operations cost, and where the cost actually is.
//!
//! Copying, moving and deleting are the only things this program does that a
//! user waits for. They also run on the event loop — nothing is drawn and no
//! key is read until they return — so their duration is not "a slow operation"
//! but "a frozen terminal", and it is worth knowing which shapes of work are
//! slow rather than guessing.
//!
//! Every group asks the same question in two shapes, because they have
//! different bottlenecks and the same total size tells you nothing about
//! which one you are in:
//!
//! - **Many small files.** One `open`, one `close` and a directory entry each;
//!   the time is syscalls, not bytes.
//! - **One large file.** A single stream; the time is bytes, not syscalls.
//!
//! Setup is kept out of the measurement with `iter_batched`: building a tree
//! of a thousand files takes far longer than copying it, and timing the two
//! together would measure the harness.

use criterion::{criterion_group, criterion_main};

#[path = "ops_bench/interactive.rs"]
mod interactive;
#[path = "ops_bench/transfer.rs"]
mod transfer;
#[path = "ops_bench/trees.rs"]
mod trees;
use interactive::{bench_directory_load, bench_editor_open, bench_unique_destination_collisions};
use transfer::{bench_copy, bench_delete, bench_move};
use trees::{many_small, source_tree, Shape};

criterion_group!(
    benches,
    bench_copy,
    bench_move,
    bench_delete,
    bench_unique_destination_collisions,
    bench_editor_open,
    bench_directory_load,
);
criterion_main!(benches);
