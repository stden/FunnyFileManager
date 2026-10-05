//! Purpose: One canonical owner per operation, and no restored handler files.

use super::*;

/// Files that define an exported (`pub` or `pub(crate)`) function, by name.
fn exported_function_owners(sources: &[SourceFile]) -> BTreeMap<String, Vec<String>> {
    let mut owners = BTreeMap::<String, Vec<String>>::new();
    for file in sources {
        for function in functions(file)
            .into_iter()
            .filter(|function| function.exported)
        {
            let files = owners.entry(function.name).or_default();
            if !files.contains(&file.relative) {
                files.push(file.relative.clone());
            }
        }
    }
    owners
}

/// Each operation is defined, exported, in exactly one canonical file.
pub fn owner_violations(sources: &[SourceFile], canonical: &[(&str, &str)]) -> Vec<String> {
    let owners = exported_function_owners(sources);
    canonical
        .iter()
        .filter_map(|(symbol, owner)| {
            let found = owners.get(*symbol).cloned().unwrap_or_default();
            (found != [*owner])
                .then(|| format!("`{symbol}` is defined in {found:?}; only in {owner}"))
        })
        .collect()
}

pub fn present_files(root: &Path, relatives: &[&str]) -> Vec<String> {
    relatives
        .iter()
        .filter(|relative| root.join(relative).exists())
        .map(|relative| format!("{relative} still exists"))
        .collect()
}

#[test]
fn owner_rule_reports_duplicate_and_missing_definitions() {
    let sources = [
        SourceFile::new(
            "src/ops/file.rs",
            "pub fn handle_copy(app: &mut App) -> io::Result<()> {\n    Ok(())\n}\n",
        ),
        SourceFile::new(
            "src/popup/copy.rs",
            "pub(crate) fn handle_copy<B>(app: &mut App) {}\n",
        ),
        SourceFile::new("src/ui/copy.rs", "fn handle_view() {}\n"),
    ];
    let canonical = [
        ("handle_copy", "src/ops/file.rs"),
        ("handle_view", "src/ops/viewer.rs"),
    ];
    assert_eq!(
        owner_violations(&sources, &canonical),
        [
            "`handle_copy` is defined in [\"src/ops/file.rs\", \"src/popup/copy.rs\"]; only in src/ops/file.rs",
            "`handle_view` is defined in []; only in src/ops/viewer.rs",
        ]
    );
}

#[test]
fn obsolete_file_rule_reports_a_restored_file() {
    let directory = tempfile::tempdir().expect("temporary tree");
    fs::create_dir_all(directory.path().join("src/handlers")).expect("handlers directory");
    fs::write(directory.path().join("src/handlers/shell.rs"), "").expect("restored file");
    assert_eq!(
        present_files(directory.path(), &OBSOLETE_HANDLER_FILES),
        ["src/handlers/shell.rs still exists"]
    );
}
