//! Purpose: Forbidden dependencies: resolved paths matched against forbidden prefixes,
//! and the violations reported.

use super::*;

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
