//! Guard test: `Panel::navigate_to` is the only way a panel changes directory.
//!
//! `navigate_to` binds the three things that must move together — path, marks
//! and selection — and rolls back when the new listing cannot be read. Four
//! call sites once hand-rolled `clear_marks(); panel.path = …; load_entries()`
//! instead (`:cd`, `:bookmark go`, and both drive-menu paths), and each one
//! stranded the panel on a directory it had no listing for as soon as the read
//! failed.
//!
//! Making the field private would enforce this in the type system, but `path`
//! is read in ~160 places across `src/` and `tests/` (integration tests cannot
//! see a `pub(crate)` field at all), so the invariant is pinned here instead —
//! the same way `tests/file_line_limit_test.rs` pins the file-size rule.

use std::fs;
use std::path::{Path, PathBuf};

/// The only file allowed to assign a panel path: the domain snapshot commit
/// invoked by `navigate_to` after the destination has been loaded.
const OWNER: &str = "models/panel.rs";

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read source directory") {
        let path = entry.expect("directory entry").path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

/// Net brace balance of one line: `+1` for an opened block, `-1` for a closed.
fn brace_balance(line: &str) -> i32 {
    let opened = i32::try_from(line.matches('{').count()).unwrap_or(i32::MAX);
    let closed = i32::try_from(line.matches('}').count()).unwrap_or(i32::MAX);
    opened - closed
}

/// Assignments of the form `<something>panel.path = …`, skipping comments and
/// inline test modules (`mod tests { … }`, `mod editor_tests { … }`).
fn panel_path_assignments(source: &str) -> Vec<(usize, String)> {
    let mut offenders = Vec::new();
    let mut test_mod_depth = 0_i32;

    for (index, line) in source.lines().enumerate() {
        if test_mod_depth > 0 {
            test_mod_depth += brace_balance(line);
            continue;
        }
        let trimmed = line.trim_start();
        if trimmed.starts_with("mod ") && trimmed.contains("test") && line.contains('{') {
            test_mod_depth = brace_balance(line);
            continue;
        }
        if trimmed.starts_with("//") {
            continue;
        }
        let squeezed = line.replace(' ', "");
        if squeezed.contains(".path=")
            && !squeezed.contains(".path==")
            && squeezed.to_lowercase().contains("panel")
        {
            offenders.push((index + 1, trimmed.to_owned()));
        }
    }
    offenders
}

#[test]
fn no_production_code_assigns_a_panel_path_directly() {
    let src = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    collect_rs_files(&src, &mut files);
    assert!(!files.is_empty(), "no .rs files found under src/");

    let mut offenders: Vec<String> = files
        .iter()
        .filter(|path| {
            // Any file whose name says "test" is test code (sibling `tests*.rs`
            // modules, `*_unit_tests.rs`, the test-support helpers), and
            // `panel.rs` owns the field, so its rollback is the one legitimate
            // write. Test code is allowed to fabricate a panel pointing at an
            // unreadable path — that is how the error paths get exercised.
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            !name.contains("test")
                && !path
                    .components()
                    .any(|component| component.as_os_str() == "tests")
                && !path.ends_with(OWNER)
        })
        .flat_map(|path| {
            let source = fs::read_to_string(path).expect("read source file");
            panel_path_assignments(&source)
                .into_iter()
                .map(move |(line, text)| format!("{}:{line}: {text}", path.display()))
        })
        .collect();
    offenders.sort();

    assert!(
        offenders.is_empty(),
        "panels must change directory through Panel::navigate_to, which drops \
         the name-keyed marks and rolls back a failed listing:\n{}",
        offenders.join("\n")
    );
}

/// The scanner has to see through the shapes these call sites actually had, and
/// must not fire on reads, comparisons or the test modules living beside them.
#[test]
fn the_scanner_recognises_assignments_but_not_reads() {
    let source = "\
fn navigate() {
    panel.path = new_path;
    app.get_active_panel().path = new_path;
    app.left_panel.path=new_path;
    let here = panel.path.clone();
    if panel.path == other.path {}
    // panel.path = commented_out;
}
mod tests {
    fn setup() {
        app.left_panel.path = temp.path().to_path_buf();
    }
}
";
    let found = panel_path_assignments(source);
    let lines: Vec<usize> = found.iter().map(|(line, _)| *line).collect();
    assert_eq!(lines, vec![2, 3, 4], "found: {found:?}");
}
