//! Purpose: Keep the MCP server a separate process whose file access stays root-confined.
//!
//! What is enforced is what the source shows:
//! - neither crate depends on the other (their lockfiles resolve every
//!   dependency, of any kind and for any target);
//! - the only MCP source the TUI compiles is `mcp/src/tool_names.rs`, and that
//!   file names nothing else in its crate;
//! - only the server composition (`main.rs`) calls the confinement API in
//!   `paths.rs`, and every extracted tool handed the server validates its path
//!   with `server.safe_path*` before anything else;
//! - modules that name `std::fs` are listed below with how their paths stay
//!   inside the root.
//!
//! Not seen: `Path` methods that reach the filesystem without naming `std::fs`
//! (`exists`, `is_dir`, `canonicalize`, `read_dir`, `metadata`).

use super::architecture::{
    dependencies, forbidden_dependencies, included_files, package_sources, starts_with_path,
    SourceFile,
};
use std::fs;
use syn::visit::{self, Visit};
use syn::{Block, ExprMethodCall, FnArg, Ident, Item, Signature};

/// The server composition: it resolves the root and owns `safe_path*`.
const SERVER: &str = "mcp/src/main.rs";
const CONFINEMENT: &str = "mcp/src/paths.rs";
const SHARED_SOURCE: &str = "mcp/src/tool_names.rs";

/// Every module that names the filesystem API, and why its paths stay inside the root.
const FILESYSTEM_MODULES: [&str; 12] = [
    SERVER,                       // every tool path passes `safe_path*` first
    CONFINEMENT,                  // canonicalizes candidates against the root
    "mcp/src/fsops.rs",           // acts on paths the server validated
    "mcp/src/kdbx.rs",            // opens the vault path the server validated
    "mcp/src/tree_walk.rs",       // walks a directory its caller validated
    "mcp/src/s3_profiles.rs",     // reads only `~/.aws` profile files, never a caller path
    "mcp/src/cleanup.rs",         // tool entries validate first (checked below)
    "mcp/src/content_search.rs",  // tool entries validate first (checked below)
    "mcp/src/directory_tools.rs", // tool entries validate first (checked below)
    "mcp/src/pattern_search.rs",  // tool entries validate first (checked below)
    "mcp/src/replace.rs",         // tool entries validate first (checked below)
    "mcp/src/reports.rs",         // tool entries validate first (checked below)
];

const FILESYSTEM_API: [&str; 4] = [
    "std::fs",
    "std::os::unix::fs",
    "std::os::windows::fs",
    "tokio::fs",
];

#[expect(
    clippy::panic,
    reason = "test support: a file the checker cannot read fails the test that asked for it"
)]
fn read(relative: &str) -> String {
    fs::read_to_string(super::repository().join(relative))
        .unwrap_or_else(|error| panic!("read {relative}: {error}"))
}

fn mcp_sources() -> Vec<SourceFile> {
    package_sources(super::repository(), "mcp")
}

/// Package names a `Cargo.lock` resolves.
fn locked_packages(lock: &str) -> Vec<&str> {
    lock.lines()
        .filter_map(|line| line.strip_prefix("name = \"")?.strip_suffix('"'))
        .collect()
}

fn cross_dependencies(tui_lock: &str, mcp_lock: &str) -> Vec<String> {
    let mut messages = Vec::new();
    if locked_packages(tui_lock).contains(&"ffm-mcp") {
        messages.push("Cargo.lock resolves `ffm-mcp`: the TUI must not link the server".to_owned());
    }
    if locked_packages(mcp_lock).contains(&"funny-file-manager") {
        messages.push(
            "mcp/Cargo.lock resolves `funny-file-manager`: the server must not link the TUI"
                .to_owned(),
        );
    }
    messages
}

/// The TUI compiles no MCP source but the import-free tool list.
fn shared_source_violations(tui: &[SourceFile], tool_names: &SourceFile) -> Vec<String> {
    let mut messages = Vec::new();
    for file in tui {
        for (line, included) in included_files(file) {
            if included.contains("mcp/") && !included.ends_with(SHARED_SOURCE) {
                messages.push(format!(
                    "{}:{line}: compiles MCP source `{included}`",
                    file.relative
                ));
            }
        }
    }
    for item in tool_names.syntax().items {
        if matches!(item, Item::Use(_) | Item::Mod(_)) {
            messages.push(format!(
                "{}: declares a `use` or `mod`",
                tool_names.relative
            ));
        }
    }
    let local = dependencies(tool_names)
        .into_iter()
        .filter(|dependency| starts_with_path(&dependency.path, "crate"));
    messages.extend(local.map(|dependency| {
        format!(
            "{}:{}: names `{}`",
            tool_names.relative, dependency.line, dependency.path
        )
    }));
    messages
}

fn filesystem_violations(sources: &[SourceFile]) -> Vec<String> {
    let unlisted = |file: &SourceFile| {
        !file.is_test() && !FILESYSTEM_MODULES.contains(&file.relative.as_str())
    };
    super::messages(&forbidden_dependencies(sources, unlisted, &FILESYSTEM_API))
}

fn confinement_api_violations(sources: &[SourceFile]) -> Vec<String> {
    let outside = |file: &SourceFile| {
        !file.is_test() && file.relative != SERVER && file.relative != CONFINEMENT
    };
    super::messages(&forbidden_dependencies(sources, outside, &["crate::paths"]))
}

/// Identifiers in a syntax node.
#[derive(Default)]
struct Idents(Vec<String>);

impl<'ast> Visit<'ast> for Idents {
    fn visit_ident(&mut self, ident: &'ast Ident) {
        self.0.push(ident.to_string());
    }
}

/// Names of the methods called in a syntax node.
#[derive(Default)]
struct MethodCalls(Vec<String>);

impl<'ast> Visit<'ast> for MethodCalls {
    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        self.0.push(call.method.to_string());
        visit::visit_expr_method_call(self, call);
    }
}

fn takes_server(signature: &Signature) -> bool {
    let mut parameter_types = Idents::default();
    for input in &signature.inputs {
        if let FnArg::Typed(typed) = input {
            parameter_types.visit_type(&typed.ty);
        }
    }
    parameter_types.0.iter().any(|name| name == "FfmServer")
}

fn starts_by_validating(body: &Block) -> bool {
    let mut first_calls = MethodCalls::default();
    if let Some(first) = body.stmts.first() {
        first_calls.visit_stmt(first);
    }
    first_calls
        .0
        .iter()
        .any(|method| method.starts_with("safe_path"))
}

/// A top-level function handed `&FfmServer` starts by calling `safe_path*`.
fn unvalidated_tool_violations(sources: &[SourceFile]) -> Vec<String> {
    let mut messages = Vec::new();
    for file in sources
        .iter()
        .filter(|file| !file.is_test() && file.relative != SERVER)
    {
        for item in file.syntax().items {
            let Item::Fn(function) = item else { continue };
            if takes_server(&function.sig) && !starts_by_validating(&function.block) {
                let name = &function.sig.ident;
                let line = name.span().start().line;
                messages.push(format!(
                    "{}:{line}: `{name}` is handed the server but does not start with `server.safe_path*`",
                    file.relative
                ));
            }
        }
    }
    messages
}

#[test]
fn tui_and_mcp_crates_do_not_depend_on_each_other() {
    let violations = cross_dependencies(&read("Cargo.lock"), &read("mcp/Cargo.lock"));
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn tui_compiles_only_the_import_free_tool_list() {
    let mcp = mcp_sources();
    let tool_names = mcp
        .iter()
        .find(|file| file.relative == SHARED_SOURCE)
        .expect("tool list");
    let violations = shared_source_violations(&super::tui_sources(), tool_names);
    assert_eq!(violations, Vec::<String>::new());
}

#[test]
fn mcp_file_access_goes_through_the_root_confinement() {
    let sources = mcp_sources();
    assert_eq!(filesystem_violations(&sources), Vec::<String>::new());
    assert_eq!(confinement_api_violations(&sources), Vec::<String>::new());
    assert_eq!(unvalidated_tool_violations(&sources), Vec::<String>::new());
}

#[test]
fn cross_dependency_rule_reads_both_lockfiles() {
    let tui = "[[package]]\nname = \"ffm-mcp\"\nversion = \"0.2.0\"\n";
    let mcp = "[[package]]\nname = \"funny-file-manager\"\nversion = \"0.1.0\"\n";
    assert_eq!(
        cross_dependencies(tui, mcp),
        [
            "Cargo.lock resolves `ffm-mcp`: the TUI must not link the server",
            "mcp/Cargo.lock resolves `funny-file-manager`: the server must not link the TUI",
        ]
    );
    assert!(cross_dependencies("name = \"ffm-mcp-client\"\n", "").is_empty());
}

#[test]
fn shared_source_rule_reports_includes_and_imports() {
    let tui = [SourceFile::new(
        "src/ops/viewer.rs",
        "include!(\"../../mcp/src/tool_names.rs\");\n\n#[path = \"../../mcp/src/paths.rs\"]\nmod paths;\n\nconst HELP: &str = include_str!(\"../../mcp/MCP.md\");\n",
    )];
    let tool_names = SourceFile::new(
        SHARED_SOURCE,
        "use crate::paths::ROOT_ENV;\n\npub const TOOL_NAMES: &[&str] = &[crate::NAME];\n",
    );
    assert_eq!(
        shared_source_violations(&tui, &tool_names),
        [
            "src/ops/viewer.rs:3: compiles MCP source `../../mcp/src/paths.rs`",
            "src/ops/viewer.rs:6: compiles MCP source `../../mcp/MCP.md`",
            "mcp/src/tool_names.rs: declares a `use` or `mod`",
            "mcp/src/tool_names.rs:1: names `crate::paths::ROOT_ENV`",
            "mcp/src/tool_names.rs:3: names `crate::NAME`",
        ]
    );
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

#[test]
fn tool_rule_reports_validation_that_is_not_the_first_statement() {
    let sources = [
        SourceFile::new(
            "mcp/src/grep.rs",
            "pub(super) fn grep(server: &FfmServer, params: GrepParams) -> Result<(), McpError> {\n    let hits = scan(&params.path);\n    let path = server.safe_path(&params.path)?;\n    Ok(())\n}\n",
        ),
        SourceFile::new(
            "mcp/src/reports.rs",
            "pub(super) fn analyse(server: &FfmServer, params: Params) -> Result<(), McpError> {\n    let root = server.safe_path(&params.path)?;\n    Ok(())\n}\n",
        ),
    ];
    assert_eq!(
        unvalidated_tool_violations(&sources),
        ["mcp/src/grep.rs:1: `grep` is handed the server but does not start with `server.safe_path*`"]
    );
}
