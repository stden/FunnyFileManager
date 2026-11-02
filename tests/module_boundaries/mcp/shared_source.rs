//! Purpose: The one MCP source the TUI compiles names nothing else in its crate.

use super::*;

/// The TUI compiles no MCP source but the import-free tool list.
pub fn shared_source_violations(tui: &[SourceFile], tool_names: &SourceFile) -> Vec<String> {
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
