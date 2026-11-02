//! Purpose: Every configured context and menu action has a stable automation identity.

use std::collections::BTreeSet;

use super::*;

#[test]
fn all_named_actions_are_unique_resolvable_and_documented() {
    let catalogue = entries();
    let rows = catalogue.as_array().unwrap();
    assert_eq!(rows.len(), expected_action_count());
    let mut identities = BTreeSet::new();
    for row in rows {
        let context = row["context"].as_str().unwrap();
        let action = row["action"].as_str().unwrap();
        assert!(identities.insert((context, action)));
        let invocation = resolve(context, action).unwrap();
        assert_eq!(invocation.context().name(), context);
        assert_ne!(row["summary"].as_str().unwrap(), "");
        assert!(row["default_keys"].is_array());
    }
    let contexts: BTreeSet<_> = identities.iter().map(|(context, _)| *context).collect();
    assert_eq!(
        contexts,
        Context::ALL.map(Context::name).into_iter().collect()
    );
    assert!(resolve("panels", "space_analysis").is_ok());
    assert!(resolve("panels", "things_catalog").is_ok());
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
fn every_menu_action_can_be_resolved_even_without_a_key() {
    for heading in crate::menu::Heading::ALL {
        for entry in crate::menu::Menu::open(heading).entries() {
            let Some(action) = entry.action() else {
                continue;
            };
            let row = tables::PANELS
                .rows()
                .find(|row| row.command == action)
                .unwrap();
            assert!(matches!(
                resolve("panels", row.name.as_str()),
                Ok(Invocation::Panels(actual)) if actual == action
            ));
        }
    }
}

#[test]
fn unknown_or_mismatched_names_are_rejected() {
    assert!(matches!(
        resolve("unknown", "save"),
        Err(LookupError::UnknownContext)
    ));
    assert!(matches!(
        resolve("editor", "unknown"),
        Err(LookupError::UnknownAction)
    ));
    assert!(matches!(
        resolve("viewer", "save"),
        Err(LookupError::UnknownAction)
    ));
    assert!(resolve("EDITOR", "save").is_err());
    assert!(resolve("editor", "Save").is_err());
}
