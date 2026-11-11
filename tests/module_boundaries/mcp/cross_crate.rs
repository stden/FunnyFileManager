//! Purpose: Neither crate's lockfile may resolve the other crate.

/// Package names a `Cargo.lock` resolves.
fn locked_packages(lock: &str) -> Vec<&str> {
    lock.lines()
        .filter_map(|line| line.strip_prefix("name = \"")?.strip_suffix('"'))
        .collect()
}

pub fn cross_dependencies(tui_lock: &str, mcp_lock: &str) -> Vec<String> {
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
    assert_eq!(
        cross_dependencies("name = \"ffm-mcp-client\"\n", "").len(),
        0
    );
}
