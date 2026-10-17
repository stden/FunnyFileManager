//! Purpose: Full-file syntax highlighting for large Java and Python sources.

use super::sources::samples;
use criterion::{BenchmarkId, Criterion};
use funny_file_manager::highlight::Highlighter;
use std::{hint::black_box, path::Path};

pub fn bench(c: &mut Criterion) {
    let mut group = c.benchmark_group("editor_languages/highlight");
    for sample in samples() {
        let id = BenchmarkId::new(sample.name, sample.line_count());
        group.bench_with_input(id, &sample, |b, sample| {
            let lines = split(&sample.source);
            b.iter(|| highlight(sample.file, &lines));
        });
    }
    group.finish();
}

fn split(source: &str) -> Vec<String> {
    source.lines().map(str::to_owned).collect()
}

fn highlight(file: &str, lines: &[String]) {
    let mut highlighter = Highlighter::for_file(Path::new(file), lines);
    for line in lines {
        black_box(highlighter.line(line));
    }
}
