//! Purpose: Syntax-aware source model shared by the architecture tests.
//!
//! Sources are parsed with `syn`, so comments, doc comments and string
//! literals never count as code. Every path a file names (in `use`
//! declarations, expressions, types, patterns, trait bounds and macro
//! invocations) is resolved to where it points: use-groups and renames
//! are expanded, and `crate::`, `super::` and `self::` are resolved against
//! the file's module path, including inline `mod` blocks.
//!
//! # Limits
//!
//! - **Attributes** (`cfg`, `doc`, `derive`, ...) are not scanned.
//! - **Macros:** a macro's input is scanned as tokens for `a::b` sequences;
//!   paths a macro builds from fragments, or pulls in from another file
//!   (`include!`), are invisible.
//! - **Name resolution:** only names imported or declared in the same file
//!   resolve. A re-export in another module (`pub use crate::ops as io;`
//!   used elsewhere as `io::x`) or a glob import (`use super::*;`) hides the
//!   original path; the glob itself is reported as `prefix::*`. The one
//!   exception is the crate root: [`forbidden_dependencies`] follows its
//!   imports (`crate::run_app` is `crate::application::run_app`, and in
//!   the MCP server `crate::fs` is `std::fs`).
//! - **cfg:** items marked exactly `#[cfg(test)]` are skipped as test code;
//!   every other configuration, including `cfg(windows)` and features, is
//!   checked as if enabled. Test files are recognized by name (see
//!   [`SourceFile::is_test`]).
//! - **Module paths** come from file locations; a `#[path]` attribute on a
//!   `mod` declaration is not followed. Build scripts are not scanned.

mod items;
mod resolve;
mod tokens;

pub use items::{facade_violations, functions, included_files, test_attribute_lines};
pub use resolve::dependencies;
use resolve::root_imports;

use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};

/// One Rust source file and the module path it compiles as.
pub struct SourceFile {
    /// Path relative to the repository root, with `/` separators.
    pub relative: String,
    /// Module path below the crate root: `["ui", "panel"]` for `src/ui/panel.rs`.
    pub module: Vec<String>,
    pub text: String,
}

impl SourceFile {
    pub fn new(relative: &str, text: &str) -> Self {
        Self {
            relative: relative.to_owned(),
            module: module_path(relative),
            text: text.to_owned(),
        }
    }

    /// Parse the file; a source the checker cannot read fails the test.
    #[expect(
        clippy::panic,
        reason = "test support: a source the checker cannot parse fails the test that asked for it"
    )]
    pub fn syntax(&self) -> syn::File {
        syn::parse_file(&self.text)
            .unwrap_or_else(|error| panic!("parse {}: {error}", self.relative))
    }

    /// `src/<module>.rs` or any file below `src/<module>/`.
    pub fn in_module(&self, name: &str) -> bool {
        self.module.first().is_some_and(|first| first == name)
    }

    /// Unit-test files: `tests*.rs`, `test_*.rs`, `*_tests.rs`, or below `tests/`.
    pub fn is_test(&self) -> bool {
        let mut components = self.relative.split('/').rev();
        let stem = components
            .next()
            .unwrap_or_default()
            .trim_end_matches(".rs");
        components.any(|component| component == "tests")
            || stem.starts_with("tests")
            || stem.starts_with("test_")
            || stem.ends_with("_tests")
    }
}

/// Every Rust file below `<root>/<package>/src`, sorted by path.
#[expect(
    clippy::expect_used,
    clippy::panic,
    reason = "test support: a source tree the checker cannot read fails the test that asked for it"
)]
pub fn package_sources(root: &Path, package: &str) -> Vec<SourceFile> {
    let mut paths = Vec::new();
    rust_files(&root.join(package).join("src"), &mut paths);
    paths.sort();
    paths
        .into_iter()
        .map(|path| {
            let relative = path
                .strip_prefix(root)
                .expect("source below the repository root")
                .to_string_lossy()
                .replace('\\', "/");
            let text = fs::read_to_string(&path)
                .unwrap_or_else(|error| panic!("read {relative}: {error}"));
            SourceFile::new(&relative, &text)
        })
        .collect()
}

#[expect(
    clippy::expect_used,
    reason = "test support: a source tree the checker cannot read fails the test that asked for it"
)]
fn rust_files(directory: &Path, files: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(directory).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            rust_files(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}

/// `src/ui/panel.rs` -> `["ui", "panel"]`; `src/ops/mod.rs` -> `["ops"]`;
/// `src/lib.rs` and `src/main.rs` are the crate root.
fn module_path(relative: &str) -> Vec<String> {
    let below_src = relative
        .split_once("src/")
        .map_or(relative, |(_, rest)| rest)
        .trim_end_matches(".rs");
    let mut module = below_src.split('/').map(str::to_owned).collect::<Vec<_>>();
    if module.last().is_some_and(|last| last == "mod") {
        module.pop();
    }
    if matches!(module.as_slice(), [root] if root == "lib" || root == "main") {
        module.clear();
    }
    module
}

/// `path` equals `prefix` or continues it with `::`.
pub fn starts_with_path(path: &str, prefix: &str) -> bool {
    path_after(path, prefix).is_some()
}

/// What follows `prefix` in `path` (`""` or `::rest`), when `path` starts with it.
fn path_after<'a>(path: &'a str, prefix: &str) -> Option<&'a str> {
    path.strip_prefix(prefix)
        .filter(|rest| rest.is_empty() || rest.starts_with("::"))
}

/// A dependency that falls under a forbidden path prefix.
#[derive(Debug, PartialEq, Eq)]
pub struct Violation {
    pub file: String,
    pub line: usize,
    /// Resolved, and followed through the crate root's imports.
    pub path: String,
    pub prefix: String,
}

impl fmt::Display for Violation {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            file,
            line,
            path,
            prefix,
        } = self;
        write!(formatter, "{file}:{line}: `{path}` (forbidden: `{prefix}`)")
    }
}

/// Dependencies of the `checked` files in `crate_sources` under a `forbidden` prefix.
///
/// `crate_sources` is the whole package: the imports of its crate roots
/// (`src/lib.rs`, `src/main.rs`) map `crate::name` back to where `name` is
/// defined. A name only one root imports cannot compile from the other
/// crate's modules, so merging both roots never flags code that builds.
pub fn forbidden_dependencies(
    crate_sources: &[SourceFile],
    checked: impl Fn(&SourceFile) -> bool,
    forbidden: &[&str],
) -> Vec<Violation> {
    let imports = crate_sources
        .iter()
        .filter(|file| file.module.is_empty())
        .flat_map(root_imports)
        .collect::<Vec<_>>();
    let mut violations = Vec::new();
    for file in crate_sources.iter().filter(|file| checked(file)) {
        for dependency in dependencies(file) {
            let path = through_root_imports(&dependency.path, &imports);
            if let Some(prefix) = forbidden
                .iter()
                .find(|prefix| starts_with_path(&path, prefix))
            {
                violations.push(Violation {
                    file: file.relative.clone(),
                    line: dependency.line,
                    path,
                    prefix: (*prefix).to_owned(),
                });
            }
        }
    }
    violations
}

/// `crate::run_app::X` -> `crate::application::run_app::X` for a name the crate root imports.
fn through_root_imports(path: &str, imports: &[(String, String)]) -> String {
    imports
        .iter()
        .find_map(|(alias, target)| path_after(path, alias).map(|rest| format!("{target}{rest}")))
        .unwrap_or_else(|| path.to_owned())
}
