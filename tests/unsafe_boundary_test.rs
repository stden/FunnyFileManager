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
        "Win32 drive mask, file attributes, registry strings and the environment broadcast",
    ),
    ("src/recording/tty.rs", "poll(2) on the terminal descriptor"),
    ("src/windows.rs", "job objects that own LSP process trees"),
    ("src/wsl/process.rs", "the Windows directory lookup"),
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

/// One piece of unsafe code, or a permission for it.
#[derive(Debug)]
struct Site {
    line: usize,
    what: String,
}

#[derive(Default)]
struct Finder {
    sites: Vec<Site>,
}

impl Finder {
    fn record(&mut self, span: Span, what: impl Into<String>) {
        self.sites.push(Site {
            line: span.start().line,
            what: what.into(),
        });
    }
}

impl<'ast> Visit<'ast> for Finder {
    fn visit_expr_unsafe(&mut self, node: &'ast syn::ExprUnsafe) {
        self.record(node.unsafe_token.span, "unsafe block");
        visit::visit_expr_unsafe(self, node);
    }

    fn visit_signature(&mut self, node: &'ast syn::Signature) {
        if let syn::Safety::Unsafe(token) = node.safety {
            self.record(token.span, format!("unsafe fn {}", node.ident));
        }
        visit::visit_signature(self, node);
    }

    fn visit_item_impl(&mut self, node: &'ast syn::ItemImpl) {
        if let Some(token) = node.unsafety {
            self.record(token.span, "unsafe impl");
        }
        visit::visit_item_impl(self, node);
    }

    fn visit_item_trait(&mut self, node: &'ast syn::ItemTrait) {
        if let Some(token) = node.unsafety {
            self.record(token.span, format!("unsafe trait {}", node.ident));
        }
        visit::visit_item_trait(self, node);
    }

    fn visit_item_foreign_mod(&mut self, node: &'ast syn::ItemForeignMod) {
        self.record(node.abi.extern_token.span, "extern block");
        visit::visit_item_foreign_mod(self, node);
    }

    fn visit_attribute(&mut self, node: &'ast syn::Attribute) {
        if permits_unsafe_code(node) {
            self.record(node.span(), "allow(unsafe_code)");
        }
        visit::visit_attribute(self, node);
    }

    /// syn does not parse macro bodies, so an `unsafe` inside one is found by
    /// its tokens.
    fn visit_macro(&mut self, node: &'ast syn::Macro) {
        let words = idents(node.tokens.clone());
        if words
            .iter()
            .any(|word| word == "unsafe" || word == "unsafe_code")
        {
            self.record(node.span(), "macro that mentions unsafe");
        }
        visit::visit_macro(self, node);
    }
}

/// Every identifier in `tokens`, inside groups too. Literals and comments are
/// not identifiers, so a string saying "unsafe" is not a finding.
fn idents(tokens: TokenStream) -> Vec<String> {
    tokens
        .into_iter()
        .flat_map(|tree| match tree {
            TokenTree::Ident(ident) => vec![ident.to_string()],
            TokenTree::Group(group) => idents(group.stream()),
            TokenTree::Punct(_) | TokenTree::Literal(_) => Vec::new(),
        })
        .collect()
}

/// `allow`, `expect` or `warn` of `unsafe_code`, directly or via `cfg_attr`.
fn permits_unsafe_code(attribute: &syn::Attribute) -> bool {
    let syn::Meta::List(list) = &attribute.meta else {
        return false;
    };
    let mut words = idents(list.tokens.clone());
    words.extend(list.path.get_ident().map(ToString::to_string));
    let names_lint = words.iter().any(|word| word == "unsafe_code");
    names_lint
        && words
            .iter()
            .any(|word| matches!(word.as_str(), "allow" | "expect" | "warn"))
}

/// Every unsafe site in `source`, which is named `path` in a parse failure.
#[expect(
    clippy::panic,
    reason = "a source the scan cannot parse fails the test; unparsable_source_fails_loudly pins it"
)]
fn sites_in(path: &str, source: &str) -> Vec<Site> {
    let file = syn::parse_file(source).unwrap_or_else(|error| panic!("parse {path}: {error}"));
    let mut finder = Finder::default();
    finder.visit_file(&file);
    finder.sites
}

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

#[expect(
    clippy::expect_used,
    reason = "test support: a source tree the scan cannot read fails the test that asked for it"
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

/// `(repository-relative path, unsafe sites)` for every file under `roots`.
#[expect(
    clippy::expect_used,
    reason = "test support: a source tree the scan cannot read fails the test that asked for it"
)]
fn scan(roots: &[&str]) -> Vec<(String, Vec<Site>)> {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for root in roots {
        rust_files(&repository.join(root), &mut files);
    }
    files.sort();
    files
        .iter()
        .map(|file| {
            let relative = file.strip_prefix(repository).expect("below the repository");
            let relative = relative.to_string_lossy().replace('\\', "/");
            let source = fs::read_to_string(file).expect("read Rust source");
            let sites = sites_in(&relative, &source);
            (relative, sites)
        })
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

#[test]
fn unsafe_in_a_models_file_fails_even_with_its_allow() {
    let source = "#[allow(unsafe_code)]\nfn read() -> u8 {\n    unsafe { *PTR }\n}\n";
    let found = violations("src/models/fixture.rs", &sites_in("fixture", source));
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].starts_with("src/models/fixture.rs:1: allow(unsafe_code)"));
    assert!(found[1].starts_with("src/models/fixture.rs:3: unsafe block"));
    assert!(found.iter().all(|message| message.contains("forbidden")));
}

#[test]
fn an_allowlisted_boundary_passes() {
    let source = "#[allow(unsafe_code)]\nfn drives() -> u32 {\n    unsafe { mask() }\n}\n";
    let sites = sites_in("fixture", source);
    assert_eq!(sites.len(), 2, "{sites:?}");
    assert!(violations("src/platform/windows.rs", &sites).is_empty());
}

#[test]
fn an_allow_outside_the_allowlist_fails() {
    let sites = sites_in("fixture", "#[allow(unsafe_code)]\nfn nothing_yet() {}\n");
    let found = violations("src/utils.rs", &sites);
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("allow(unsafe_code)"), "{found:?}");
    assert!(found[0].contains("not a reviewed"), "{found:?}");
}

#[test]
fn a_file_without_unsafe_passes_anywhere() {
    let sites = sites_in("fixture", "fn safe() -> u8 { 1 }\n");
    assert!(violations("src/models/fixture.rs", &sites).is_empty());
    assert!(violations("mcp/src/main.rs", &sites).is_empty());
}

#[test]
fn every_unsafe_construct_is_found() {
    let cases = [
        ("unsafe fn raw() {}", "unsafe fn raw"),
        ("unsafe impl Send for Handle {}", "unsafe impl"),
        ("unsafe trait Raw {}", "unsafe trait Raw"),
        ("extern \"C\" { fn c(); }", "extern block"),
        ("unsafe extern \"C\" { fn c(); }", "extern block"),
        ("#![allow(unsafe_code)]", "allow(unsafe_code)"),
        ("#[expect(unsafe_code)] fn f() {}", "allow(unsafe_code)"),
        (
            "#[cfg_attr(test, allow(unsafe_code))] fn f() {}",
            "allow(unsafe_code)",
        ),
        ("impl T { unsafe fn m(&self) {} }", "unsafe fn m"),
        (
            "macro_rules! m { () => { unsafe { f() } } }",
            "macro that mentions unsafe",
        ),
        (
            "fn f() { run!(unsafe { g() }); }",
            "macro that mentions unsafe",
        ),
    ];
    for (source, expected) in cases {
        let sites = sites_in("fixture", source);
        assert!(
            sites.iter().any(|site| site.what == expected),
            "{source:?} should report {expected:?}, got {sites:?}"
        );
    }
}

#[test]
fn lookalikes_are_not_findings() {
    let cases = [
        "#![forbid(unsafe_code)]",
        "#[deny(unsafe_code)] fn f() {}",
        "const WORDS: &[&str] = &[\"unsafe\", \"unsafe_code\"];",
        "// unsafe { f() }\nfn f() {}",
        "fn f() { assert!(text.contains(\"unsafe\")); }",
        "type Callback = Option<unsafe extern \"C\" fn(i32)>;",
    ];
    for source in cases {
        let sites = sites_in("fixture", source);
        assert!(sites.is_empty(), "{source:?} reported {sites:?}");
    }
}

#[test]
fn forbidden_layers_match_whole_module_names() {
    assert!(in_forbidden_layer("src/models.rs"));
    assert!(in_forbidden_layer("src/models/drive_entry.rs"));
    assert!(in_forbidden_layer("src/ui/viewer.rs"));
    assert!(in_forbidden_layer("src/command.rs"));
    assert!(!in_forbidden_layer("src/usermenu.rs"));
    assert!(!in_forbidden_layer("src/ops/command.rs"));
    assert!(!in_forbidden_layer("src/commands_extra.rs"));
}

#[test]
#[should_panic(expected = "parse broken.rs")]
fn unparsable_source_fails_loudly() {
    sites_in("broken.rs", "fn {");
}
