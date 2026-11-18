//! Purpose: Measure selecting a whole listing and removing its marks by mask.

use std::hint::black_box;

use criterion::{BatchSize, Criterion, Throughput};
use funny_file_manager::mask::Mask;

use super::panel;

pub fn bench_bulk_marks(c: &mut Criterion) {
    let empty = panel(2_048);
    let mut selected = empty.clone();
    selected.mark_all();
    assert_eq!(selected.marked.len(), 2_048);
    let mask = Mask::parse("*.txt");
    assert_eq!(selected.mark_matching(&mask, false), 2_048);
    selected.restore_marks();
    assert_eq!(selected.marked.len(), 2_048);

    let mut group = c.benchmark_group("panel/bulk_marks");
    group.throughput(Throughput::Elements(2_048));
    group.bench_function("select_all", |b| {
        b.iter_batched_ref(
            || empty.clone(),
            |panel| {
                panel.mark_all();
                black_box(panel.marked.len())
            },
            BatchSize::LargeInput,
        );
    });
    group.bench_function("remove_by_extension", |b| {
        b.iter_batched_ref(
            || selected.clone(),
            |panel| panel.mark_matching(black_box(&mask), false),
            BatchSize::LargeInput,
        );
    });
    group.finish();
}
