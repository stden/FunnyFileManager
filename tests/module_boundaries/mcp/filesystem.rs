//! Purpose: Modules that name the filesystem API are listed, and only the server confines paths.

use super::{forbidden_dependencies, read, Ident, SourceFile, Visit, SERVER};

const CONFINEMENT: &str = "mcp/src/paths.rs";

/// Every module that names the filesystem API, and why its paths stay inside the root.
const FILESYSTEM_MODULES: [&str; 35] = [
    SERVER,                              // every tool path passes `safe_path*` first
    CONFINEMENT,                         // canonicalizes candidates against the root
    "mcp/src/fsops.rs",                  // acts on paths the server validated
    "mcp/src/fsops/copy.rs",             // acts on paths the server validated
    "mcp/src/fsops/moving.rs",           // acts on paths the server validated
    "mcp/src/fsops/remove_moved.rs",     // acts on paths the server validated
    "mcp/src/kdbx.rs",                   // opens the vault path the server validated
    "mcp/src/tree_walk.rs",              // walks a directory its caller validated
    "mcp/src/s3_profiles.rs",            // reads only `~/.aws` profile files, never a caller path
    "mcp/src/cleanup.rs",                // tool entries validate first (checked below)
    "mcp/src/content_search.rs",         // tool entries validate first (checked below)
    "mcp/src/directory_tools.rs",        // tool entries validate first (checked below)
    "mcp/src/pattern_search.rs",         // tool entries validate first (checked below)
    "mcp/src/replace.rs",                // tool entries validate first (checked below)
    "mcp/src/reports.rs",                // tool entries validate first (checked below)
    "mcp/src/checksum.rs",               // hashes a file its caller validated
    "mcp/src/walk.rs",                   // walks what its caller validated, never a link
    "mcp/src/pattern_search/filters.rs", // tool entries validate first (checked below)
    "mcp/src/tools/analysis.rs",         // each caller path passes `safe_path*` first
    "mcp/src/tools/batch.rs",            // each entry passes `safe_path_nofollow_entry`
    "mcp/src/tools/bookmarks.rs",        // `~/.ffm/bookmarks.json`; a new bookmark is validated
    "mcp/src/tools/comparing.rs",        // each caller path passes `safe_path*` first
    "mcp/src/tools/file_stats.rs",       // each caller path passes `safe_path*` first
    "mcp/src/tools/listing.rs",          // each caller path passes `safe_path*` first
    "mcp/src/tools/permissions.rs",      // each caller path passes `safe_path*` first
    "mcp/src/tools/reading.rs",          // each caller path passes `safe_path*` first
    "mcp/src/tools/s3_transfer.rs",      // each caller path passes `safe_path*` first
    "mcp/src/tools/search.rs",           // each caller path passes `safe_path*` first
    "mcp/src/tools/symlinks.rs",         // each caller path passes `safe_path*` first
    "mcp/src/tools/system.rs",           // `/etc/hostname` and PATH entries, never a caller path
    "mcp/src/tools/tail.rs",             // each caller path passes `safe_path*` first
    "mcp/src/tools/text_format.rs",      // each caller path passes `safe_path*` first
    "mcp/src/tools/transfer.rs",         // each caller path passes `safe_path*` first
    "mcp/src/tools/utilities.rs",        // each caller path passes `safe_path*` first
    "mcp/src/tools/writing.rs",          // each caller path passes `safe_path*` first
];

const FILESYSTEM_API: [&str; 4] = [
    "std::fs",
    "std::os::unix::fs",
    "std::os::windows::fs",
    "tokio::fs",
];

pub fn filesystem_violations(sources: &[SourceFile]) -> Vec<String> {
    let unlisted = |file: &SourceFile| {
        !file.is_test() && !FILESYSTEM_MODULES.contains(&file.relative.as_str())
    };
    super::messages(&forbidden_dependencies(sources, unlisted, &FILESYSTEM_API))
}

pub fn confinement_api_violations(sources: &[SourceFile]) -> Vec<String> {
    let outside = |file: &SourceFile| {
        !file.is_test() && file.relative != SERVER && file.relative != CONFINEMENT
    };
    super::messages(&forbidden_dependencies(sources, outside, &["crate::paths"]))
}

/// Identifiers in a syntax node.
#[derive(Default)]
pub struct Idents(pub Vec<String>);

impl<'ast> Visit<'ast> for Idents {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.0.push(ident.to_string());
    }
}

#[test]
fn filesystem_rules_report_unlisted_access_even_through_the_crate_root() {
    let sources = [
        SourceFile::new("mcp/src/main.rs", "use std::fs;\n\nmod globmatch;\n"),
        SourceFile::new(
            "mcp/src/globmatch.rs",
            "fn pattern(path: &Path) -> String {\n    crate::fs::read_to_string(path).unwrap_or_default()\n}\n",
        ),
        SourceFile::new(
            "mcp/src/archive.rs",
            "use std::fs::File;\nuse crate::paths::sanitize_path;\n",
        ),
    ];
    assert_eq!(
        filesystem_violations(&sources),
        [
            "mcp/src/globmatch.rs:2: `std::fs::read_to_string` (forbidden: `std::fs`)",
            "mcp/src/archive.rs:1: `std::fs::File` (forbidden: `std::fs`)",
        ]
    );
    assert_eq!(
        confinement_api_violations(&sources),
        ["mcp/src/archive.rs:2: `crate::paths::sanitize_path` (forbidden: `crate::paths`)"]
    );
}

/// The installer writes the server's root variable into client configs, but
/// may not compile `paths.rs` (see [`shared_source_violations`]), so it keeps
/// its own copy of the name. This reads the server's declaration as text and
/// holds the two together.
#[test]
fn the_installer_names_the_root_variable_the_server_reads() {
    let declaration = format!(
        "pub const ROOT_ENV: &str = \"{}\";",
        funny_file_manager::mcp_clients::ROOT_ENV
    );
    assert!(
        read(CONFINEMENT).contains(&declaration),
        "{CONFINEMENT} must still declare `{declaration}`"
    );
}
