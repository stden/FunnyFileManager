//! Purpose: syntax highlighting, which the editor redoes for every visible line on every frame.

use criterion::Criterion;
use std::hint::black_box;

/// Syntax highlighting, which the editor redoes for every visible line on
/// every frame.
pub fn bench_highlight(c: &mut Criterion) {
    let mut group = c.benchmark_group("highlight");
    for (name, path, source) in [
        (
            "rust_screenful",
            "bench.rs",
            r#"pub fn main() { let name: String = "value".to_owned(); /* note */ }"#,
        ),
        (
            "shell_screenful",
            "bench.sh",
            r#"echo "$HOME/bin" && cd /srv && export PATH="$PATH:/opt" # note"#,
        ),
        (
            "plain_screenful",
            "bench.unknown",
            "just some words with no language at all",
        ),
    ] {
        let lines: Vec<_> = std::iter::repeat_n(source.to_owned(), 40).collect();
        group.bench_function(name, |b| {
            b.iter(|| highlight_lines(path, &lines));
        });
    }
    group.finish();
}

fn highlight_lines(path: &str, lines: &[String]) {
    let mut highlighter =
        funny_file_manager::highlight::Highlighter::for_file(std::path::Path::new(path), lines);
    for line in lines {
        black_box(highlighter.line(line));
    }
}
