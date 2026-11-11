//! Purpose: Typst highlighting cost on files large enough to fill the editor.
//!
//! The caret-at-the-bottom frame colours every line. Java and Python are
//! measured in `editor_languages`; `.typ` is not.

use std::hint::black_box;
use std::path::Path;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use funny_file_manager::highlight::Highlighter;

fn source(lines: usize) -> Vec<String> {
    (0..lines)
        .map(|index| {
            if index.is_multiple_of(5) {
                format!("#let value{index} = {index} // note")
            } else {
                format!("let name{index}: str = \"v{index}\"")
            }
        })
        .collect()
}

fn highlight(lines: &[String]) {
    let mut highlighter = Highlighter::for_file(Path::new("notes.typ"), lines);
    for line in lines {
        black_box(highlighter.line(line));
    }
}

fn bench_typst(c: &mut Criterion) {
    let mut group = c.benchmark_group("highlight/typst");
    group.sample_size(10);
    group.warm_up_time(Duration::from_millis(200));
    group.measurement_time(Duration::from_secs(1));
    for lines in [2_000_usize, 20_000] {
        let sample = source(lines);
        group.bench_with_input(BenchmarkId::from_parameter(lines), &sample, |b, sample| {
            b.iter(|| highlight(sample));
        });
    }
    group.finish();
}

criterion_group!(benches, bench_typst);
criterion_main!(benches);
