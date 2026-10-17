//! Purpose: Reading the crate: its public function names and the test text that must name them.

use super::*;

/// A file holding tests rather than production code.
fn is_test_source(path: &str) -> bool {
    path.contains("/tests/")
        || path.contains("\\tests\\")
        || Path::new(path)
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.starts_with("tests"))
}

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Names of every `pub fn` and `pub(crate) fn` declared in production sources.
pub fn public_functions(root: &Path) -> BTreeSet<String> {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut names = BTreeSet::new();
    for path in files {
        let display = path.display().to_string();
        if is_test_source(&display) {
            continue;
        }
        let Ok(text) = fs::read_to_string(&path) else {
            continue;
        };
        for line in text.lines() {
            if let Some(name) = declared_name(line.trim_start()) {
                names.insert(name);
            }
        }
    }
    names
}

/// The function name in `pub fn name(`, or `None` for any other line.
///
/// Deliberately textual: pulling in a parser to read a declaration this regular
/// would cost more than the guard is worth.
fn declared_name(line: &str) -> Option<String> {
    let rest = line
        .strip_prefix("pub(crate) ")
        .or_else(|| line.strip_prefix("pub "))?;
    let rest = rest.strip_prefix("const ").unwrap_or(rest);
    let rest = rest.strip_prefix("async ").unwrap_or(rest);
    let rest = rest.strip_prefix("unsafe ").unwrap_or(rest);
    let rest = rest.strip_prefix("fn ")?;
    let name: String = rest
        .chars()
        .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '_')
        .collect();
    (!name.is_empty()).then_some(name)
}

/// Every test source in the crate, concatenated.
pub fn test_corpus(root: &Path) -> String {
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    rust_files(&root.join("tests"), &mut files);
    files
        .iter()
        .filter(|path| is_test_source(&path.display().to_string()))
        .filter_map(|path| fs::read_to_string(path).ok())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Whether a test calls the function or names its path.
pub fn is_exercised(corpus: &str, name: &str) -> bool {
    corpus.contains(&format!("{name}(")) || corpus.contains(&format!("::{name}"))
}

/// The parser must read the declaration forms this crate actually uses.
#[test]
fn declared_name_reads_the_forms_the_crate_uses() {
    assert_eq!(declared_name("pub fn open(").as_deref(), Some("open"));
    assert_eq!(
        declared_name("pub(crate) fn touch(&mut self)").as_deref(),
        Some("touch")
    );
    assert_eq!(
        declared_name("pub const fn width() -> u16").as_deref(),
        Some("width")
    );
    assert_eq!(declared_name("fn private_helper()"), None);
    assert_eq!(declared_name("pub struct App {"), None);
    assert_eq!(declared_name("// pub fn commented_out()"), None);
}

/// A test file must be recognised wherever it lives, on either path separator.
#[test]
fn test_sources_are_recognised_by_name_and_by_directory() {
    assert!(is_test_source("src/kdbx/tests.rs"));
    assert!(is_test_source("src/ops/tests/shell_commands.rs"));
    assert!(is_test_source("src/complete/tests_engines.rs"));
    assert!(!is_test_source("src/kdbx/vault.rs"));
    assert!(!is_test_source("src/casefold.rs"));
}
