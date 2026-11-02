//! Purpose: Every public named action is discoverable with stable context and schema fields.

use funny_file_manager::keymap::{tables, Context, Table};
use std::collections::BTreeSet;

use super::*;

#[test]
fn public_catalogue_contains_all_actions_and_fourteen_unique_contexts() {
    let root = workspace();
    let mut session = Session::start(root.path());
    let mut identities = BTreeSet::new();
    let mut contexts = BTreeSet::new();
    let actions = session.actions.as_array().unwrap();
    assert_eq!(actions.len(), expected_action_count());
    for action in actions {
        let context = action["context"].as_str().unwrap();
        let name = action["action"].as_str().unwrap();
        assert!(identities.insert((context, name)));
        contexts.insert(context);
        assert_ne!(action["summary"].as_str().unwrap(), "");
        assert!(action["default_keys"].is_array());
    }
    assert_eq!(
        contexts,
        Context::ALL.map(Context::name).into_iter().collect()
    );
    assert!(identities.contains(&("panels", "system_temp_cleanup")));
    assert!(identities.contains(&("editor", "save")));
    assert!(identities.contains(&("panels", "space_analysis")));
    assert!(identities.contains(&("panels", "things_catalog")));
    session.close();
}

fn count<C>(table: &Table<C>) -> usize {
    table.rows().count()
}

fn expected_action_count() -> usize {
    [
        count(&tables::PANELS),
        count(&tables::TREE_PANEL),
        count(&tables::NETBOX_PANEL),
        count(&tables::COMMAND_LINE),
        count(&tables::COMMAND_LINE_SEARCH),
        count(&tables::COMMAND_LINE_COMPLETION),
        count(&tables::VIEWER),
        count(&tables::EDITOR),
        count(&tables::TASK_LIST),
        count(&tables::TASK_LIST_SEARCH),
        count(&tables::WSL_MANAGER),
        count(&tables::LANGUAGE_SERVERS),
        count(&tables::PATH_EDITOR),
        count(&tables::VAULT),
    ]
    .into_iter()
    .sum()
}

#[test]
fn invalid_catalogue_identity_reports_an_error_and_the_session_stays_usable() {
    let root = workspace();
    let mut session = Session::start(root.path());
    let response =
        session.request(json!({"operation":"action","context":"viewer","action":"save"}));
    assert_eq!(response["ok"], false);
    assert!(response["error"]["message"].as_str().is_some());
    let state = session.state();
    assert_eq!(state["modal"]["kind"], "none");
    assert_path(&state["panels"]["left"]["path"], root.path());
    session.close();
}
