//! Purpose: Keep unsafe code inside the reviewed platform and FFI boundaries.
//!
//! The root crate denies `unsafe_code`, so each unsafe item there opts in with
//! an item-level `#[allow(unsafe_code)]`; colorer-sys *is* the FFI crate and
//! allows it throughout; the MCP server forbids it. The compiler checks each
//! item. This test checks *where* they are: every unsafe block, fn, impl,
//! trait, extern block, `allow(unsafe_code)` and macro that mentions either
//! must sit in a file on [`BOUNDARIES`], and no boundary may live in state,
//! rendering, key handling or parsing ([`FORBIDDEN_LAYERS`]).
//!
//! # Fixing a failure
//!
//! Prefer a safe API — std, `rustix`, or a wrapper in `src/platform`. When raw
//! FFI is genuinely needed, keep it in the narrowest item, open the comment
//! above every block with `// SAFETY` followed by a colon (spelled out here, or
//! Clippy reads this doc as a safety comment on the first import), and add the
//! file to [`BOUNDARIES`] with its reason. Growing that list is a reviewed
//! decision, not a formality.

use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::{Span, TokenStream, TokenTree};
use syn::spanned::Spanned;
use syn::visit::{self, Visit};

#[path = "unsafe_boundary_test/finder.rs"]
mod finder;
#[path = "unsafe_boundary_test/fixtures.rs"]
mod fixtures;
use finder::{scan, sites_in, Site};

/// Files reviewed as unsafe boundaries, relative to the repository root.
const BOUNDARIES: &[(&str, &str)] = &[
    ("colorer-sys/src/lib.rs", "the Colorer C API and its stubs"),
    (
        "src/diskfree.rs",
        "Windows free-space and drive-type queries",
    ),
    (
        "src/directory/owner.rs",
        "reentrant passwd/group name lookups",
    ),
    (
        "src/highlight/colorer.rs",
        "the safe wrapper over colorer-sys",
    ),
    (
        "src/kdbx/secret.rs",
        "volatile zeroization of vault secrets",
    ),
    (
        "src/kdbx/tests_edit.rs",
        "reads a live form's buffer to prove the wipe",
    ),
    (
        "src/kdbx/tests_secrets.rs",
        "reads secret buffers to prove the wipe",
    ),
    ("src/ops/apply/context.rs", "Windows 8.3 short paths"),
    ("src/ops/apply/lists.rs", "Windows code-page conversion"),
    (
        "src/ops/panel_details/windows.rs",
        "Windows hard-link count",
    ),
    (
        "src/ops/shell/console.rs",
        "Windows new-console process creation",
    ),
    (
        "src/ops/shell/console/tests.rs",
        "waits on the console probe's process",
    ),
    (
        "src/platform/windows.rs",
        "Win32 drive mask, file attributes and the environment broadcast",
    ),
    (
        "src/platform/windows/registry/key.rs",
        "Win32 registry key handles and string values",
    ),
    ("src/recording/tty.rs", "poll(2) on the terminal descriptor"),
    (
        "src/windows/process_tree.rs",
        "job objects that own LSP process trees",
    ),
    ("src/wsl/process/launch.rs", "the Windows directory lookup"),
];

/// State, rendering, key handling and parsing: never a place for unsafe code.
const FORBIDDEN_LAYERS: &[&str] = &[
    "src/models",
    "src/ui",
    "src/action",
    "src/popup",
    "src/command",
];

/// Source trees scanned, relative to the repository root.
const ROOTS: &[&str] = &["src", "mcp/src", "colorer-sys/src"];

/// Whether `path` is `layer.rs` or lies under `layer/`.
fn in_forbidden_layer(path: &str) -> bool {
    FORBIDDEN_LAYERS.iter().any(|layer| {
        path.strip_prefix(layer)
            .is_some_and(|rest| rest == ".rs" || rest.starts_with('/'))
    })
}

fn is_boundary(path: &str) -> bool {
    BOUNDARIES.iter().any(|(boundary, _)| *boundary == path)
}

/// One message per site `path` may not hold; empty when it may hold them all.
fn violations(path: &str, sites: &[Site]) -> Vec<String> {
    let reason = if in_forbidden_layer(path) {
        "unsafe is forbidden in this layer"
    } else if is_boundary(path) {
        return Vec::new();
    } else {
        "not a reviewed unsafe boundary"
    };
    sites
        .iter()
        .map(|site| format!("{path}:{}: {} — {reason}", site.line, site.what))
        .collect()
}

#[test]
fn unsafe_code_stays_inside_the_reviewed_boundaries() {
    let offenders: Vec<String> = scan(ROOTS)
        .iter()
        .flat_map(|(path, sites)| violations(path, sites))
        .collect();
    assert!(
        offenders.is_empty(),
        "unsafe code outside the reviewed boundaries — use a safe API or \
         src/platform, or review the file into BOUNDARIES:\n{}",
        offenders.join("\n")
    );
}

#[test]
fn the_mcp_server_has_no_unsafe_code() {
    let sites: Vec<String> = scan(&["mcp/src"])
        .iter()
        .flat_map(|(path, sites)| {
            sites
                .iter()
                .map(move |site| format!("{path}:{}", site.line))
        })
        .collect();
    assert!(sites.is_empty(), "the MCP server forbids unsafe: {sites:?}");
}

/// A boundary that no longer holds unsafe code must leave the list, so the
/// list stays the set of files a reviewer actually has to read.
#[test]
fn every_boundary_still_holds_unsafe_code() {
    let scanned = scan(ROOTS);
    let stale: Vec<&str> = BOUNDARIES
        .iter()
        .map(|(boundary, _)| *boundary)
        .filter(|boundary| {
            !scanned
                .iter()
                .any(|(path, sites)| path == boundary && !sites.is_empty())
        })
        .collect();
    assert!(stale.is_empty(), "delete these from BOUNDARIES: {stale:?}");
}

#[test]
fn no_boundary_is_in_a_forbidden_layer_or_the_mcp_server() {
    for (boundary, reason) in BOUNDARIES {
        assert!(!in_forbidden_layer(boundary), "{boundary}: {reason}");
        assert!(!boundary.starts_with("mcp/"), "{boundary}: {reason}");
    }
}
