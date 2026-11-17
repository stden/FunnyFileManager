//! Purpose: The abort hook is installed before the terminal is activated.

use super::*;

/// The panic hook that restores the terminal is installed before the terminal changes.
pub fn abort_hook_violations(bootstrap: &SourceFile) -> Vec<String> {
    let Some(session) = functions(bootstrap)
        .into_iter()
        .find(|function| function.name == "run_session")
    else {
        return vec![format!("{}: no `run_session`", bootstrap.relative)];
    };
    let calls = dependencies(bootstrap)
        .into_iter()
        .filter(|dependency| session.lines.contains(&dependency.line))
        .collect::<Vec<_>>();
    let first = |path: &str| {
        calls
            .iter()
            .find(|dependency| dependency.path == path)
            .map(|dependency| (dependency.line, dependency.column))
    };
    let hook = first("std::panic::set_hook");
    let enter = first("crate::terminal::TerminalSession::enter");
    if matches!((hook, enter), (Some(hook), Some(enter)) if hook < enter) {
        return Vec::new();
    }
    vec![format!(
        "{}: `run_session` must call `std::panic::set_hook` (at {hook:?}) before \
         `TerminalSession::enter` (at {enter:?})",
        bootstrap.relative
    )]
}

#[test]
fn abort_hook_rule_reads_code_order_not_comments() {
    let reversed = SourceFile::new(
        "src/bootstrap.rs",
        "fn run_session() {\n    // set_hook first, then TerminalSession::enter.\n    let session = crate::terminal::TerminalSession::enter();\n    std::panic::set_hook(Box::new(|_| {}));\n}\n",
    );
    assert_eq!(
        abort_hook_violations(&reversed),
        [
            "src/bootstrap.rs: `run_session` must call `std::panic::set_hook` (at Some((4, 4))) \
          before `TerminalSession::enter` (at Some((3, 18)))"
        ]
    );
    let imported = SourceFile::new(
        "src/bootstrap.rs",
        "use crate::terminal::TerminalSession;\nuse std::panic;\n\nfn run_session() {\n    panic::set_hook(Box::new(|_| {}));\n    let session = TerminalSession::enter();\n}\n",
    );
    assert_eq!(abort_hook_violations(&imported).len(), 0);
}
