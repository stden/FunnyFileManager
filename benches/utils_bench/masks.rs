//! Purpose: Measure applying a parsed file mask to a directory listing.

use std::hint::black_box;

use criterion::{Criterion, Throughput};
use funny_file_manager::mask::Mask;

pub fn bench_file_masks(c: &mut Criterion) {
    let mut names = Vec::new();
    for index in 0..256 {
        names.extend([
            format!("report-north-region-{index:04}-v02-final.csv"),
            format!("report-süd-region-{index:04}-v12-draft.CSV"),
            format!("report-east-region-{index:04}-v03-final.csv.tmp"),
            format!("résumé-{index:04}.pdf"),
        ]);
    }
    let mut group = c.benchmark_group("file_mask");
    group.throughput(Throughput::Elements(1_024));
    for (label, source, expected) in [
        ("match_all", "*", 1_024),
        ("extension", "*.csv", 512),
        ("general_glob", "report-*-v??-final.*", 512),
    ] {
        let mask = Mask::parse(source);
        let count = |listing: &[String]| listing.iter().filter(|name| mask.matches(name)).count();
        assert_eq!(count(&names), expected);
        group.bench_function(label, |b| b.iter(|| count(black_box(&names))));
    }
    group.finish();
}
