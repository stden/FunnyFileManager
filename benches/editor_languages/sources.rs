//! Purpose: Repeatable Java and Python sources large enough to stress the editor.

use std::fmt::Write;

pub struct Sample {
    pub name: &'static str,
    pub file: &'static str,
    pub source: String,
}

impl Sample {
    #[must_use]
    pub fn line_count(&self) -> usize {
        self.source.lines().count()
    }
}

#[must_use]
pub fn samples() -> Vec<Sample> {
    [2_000_usize, 20_000]
        .into_iter()
        .flat_map(language_pair)
        .collect()
}

fn language_pair(lines: usize) -> [Sample; 2] {
    [
        language("java", "Large.java", java(lines)),
        language("python", "large.py", python(lines)),
    ]
}

fn language(name: &'static str, file: &'static str, source: String) -> Sample {
    Sample { name, file, source }
}

fn java(lines: usize) -> String {
    let mut source = String::with_capacity(lines.saturating_mul(64));
    push(&mut source, "package bench;");
    push(&mut source, "public class Large {");
    for index in 0..lines.saturating_sub(3) {
        push_java(&mut source, index);
    }
    push(&mut source, "}");
    source
}

fn python(lines: usize) -> String {
    let mut source = String::with_capacity(lines.saturating_mul(48));
    for index in 0..lines / 2 {
        push_python(&mut source, index);
    }
    source
}

#[expect(
    clippy::expect_used,
    reason = "a benchmark fixture that cannot grow is a broken run"
)]
fn push(source: &mut String, line: &str) {
    writeln!(source, "{line}").expect("benchmark source fits in memory");
}

fn push_java(source: &mut String, index: usize) {
    push(
        source,
        &format!("    public int value{index}() {{ return \"v{index}\"; }} // field"),
    );
}

fn push_python(source: &mut String, index: usize) {
    push(source, &format!("def value_{index}(name: str) -> int:"));
    push(source, &format!("    return {index}  # count"));
}
