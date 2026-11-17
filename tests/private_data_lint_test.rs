//! Purpose: keep private data out of the published repository.
//!
//! Every tracked text file is scanned for personal e-mail addresses,
//! machine-specific absolute paths, credential-shaped strings and public IP
//! addresses. Runs with the ordinary test suite, so CI refuses a commit that
//! would publish any of them.
//!
//! Site-specific words (an employer, an internal project, a host) must not be
//! spelled out here — that would publish them. Put them, one regex per line,
//! in `~/.config/ffm/private-words.txt` or the file named by
//! `FFM_PRIVATE_WORDS_FILE`; the test reads it when present and stays silent
//! when it is absent (as on CI).

use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::{Regex, RegexBuilder};

#[path = "private_data_lint_test/rules.rs"]
mod rules;
use rules::Rules;

/// Binary or generated files that carry no prose.
const SKIP_EXTENSIONS: &[&str] = &[
    "png", "jpg", "jpeg", "gif", "ico", "woff", "woff2", "ttf", "pdf", "kdbx",
];

struct Finding {
    path: PathBuf,
    line: usize,
    rule: &'static str,
    hint: String,
}

#[test]
fn tracked_files_carry_no_private_data() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let rules = Rules::new().expect("every rule pattern compiles");
    let mut findings = Vec::new();
    let tracked = tracked_files(&root).expect("git must be available to list tracked files");
    for path in tracked {
        let Ok(text) = std::fs::read_to_string(root.join(&path)) else {
            continue; // not UTF-8: a binary we did not list
        };
        for (index, line) in text.lines().enumerate() {
            rules.check(line, |rule, hint| {
                findings.push(Finding {
                    path: path.clone(),
                    line: index + 1,
                    rule,
                    hint,
                });
            });
        }
    }
    let report: Vec<String> = findings
        .iter()
        .map(|f| format!("{}:{}: {} ({})", f.path.display(), f.line, f.rule, f.hint))
        .collect();
    assert_eq!(
        findings.len(),
        0,
        "private data must not be published — fix or allow-list:\n{}",
        report.join("\n")
    );
}

fn tracked_files(root: &Path) -> io::Result<Vec<PathBuf>> {
    let output = Command::new("git")
        .args(["ls-files", "-z"])
        .current_dir(root)
        .output()?;
    assert!(output.status.success(), "git ls-files failed");
    Ok(output
        .stdout
        .split(|&b| b == 0)
        .filter(|p| !p.is_empty())
        .map(|p| PathBuf::from(String::from_utf8_lossy(p).into_owned()))
        .filter(|p| {
            !p.extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| SKIP_EXTENSIONS.contains(&e))
        })
        .collect())
}
