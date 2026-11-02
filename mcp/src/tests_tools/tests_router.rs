//! Purpose: The tool names the TUI shares match the router the server builds.

use super::*;

// ═══════════════════════════════════════════════════════════════
// The registered tool list
// ═══════════════════════════════════════════════════════════════

#[test]
fn tool_names_match_the_registered_router() {
    let mut registered: Vec<String> = FfmServer::tool_router()
        .list_all()
        .into_iter()
        .map(|tool| tool.name.into_owned())
        .collect();
    registered.sort();

    let mut declared: Vec<String> = TOOL_NAMES.iter().map(|name| (*name).to_owned()).collect();
    declared.sort();

    assert_eq!(
        TOOL_NAMES.len(),
        registered.len(),
        "TOOL_NAMES is what a status screen counts — it must match the router exactly"
    );
    assert_eq!(
        declared, registered,
        "TOOL_NAMES has drifted from the tools the router registers"
    );
}
