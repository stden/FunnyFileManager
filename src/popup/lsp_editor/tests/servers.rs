//! Purpose: removing and adding servers — Del resets a catalog server and removes an added one, and a removed server added again is saved as new.

use super::*;

#[test]
fn del_resets_a_catalog_server_and_removes_an_added_one() {
    let machine = LspMachine::new();
    machine.write_settings("lsp:\n  css: {command: c}\n  mine: {command: m}\n");
    let mut app = machine.opened();
    select(&mut app, "css");
    press(&mut app, KeyCode::Delete);
    assert_eq!(
        editor(&app).draft().0["css"].command,
        "vscode-css-language-server"
    );
    select(&mut app, "mine");
    press(&mut app, KeyCode::F(8));
    assert!(!editor(&app).draft().0.contains_key("mine"));
    press(&mut app, KeyCode::F(5));
    assert!(matches!(editor(&app).notice(), Some(Notice::Info(_))));
}

/// Del on the added `zig`, then Ins with `command` and extension `zig`.
fn remove_and_add_zig_again(app: &mut App, command: &str) {
    select(app, "zig");
    press(app, KeyCode::Delete);
    press(app, KeyCode::Insert);
    type_text(app, "zig");
    press(app, KeyCode::Tab);
    type_text(app, command);
    press(app, KeyCode::Tab);
    press(app, KeyCode::Tab);
    type_text(app, "zig");
    press(app, KeyCode::Enter);
    assert_eq!(mode(app), &Mode::Browse, "{command}: the form closed");
}

/// Del then Ins on an added server's language makes a new server: F2 writes
/// it whole, and re-entering the old command and extension still saves.
#[test]
fn a_removed_server_added_again_is_saved_as_the_new_one() {
    for command in ["zls2", "zls"] {
        let machine = LspMachine::new();
        machine.write_settings(
            "lsp:\n  zig: {command: zls, extensions: [zig], root_markers: [build.zig], custom: 1}\n",
        );
        let mut app = machine.opened();
        remove_and_add_zig_again(&mut app, command);
        press(&mut app, KeyCode::F(2));
        assert!(
            !editor(&app).dirty(),
            "{command}: {:?}",
            editor(&app).notice()
        );
        let file = machine.parsed();
        let zig = &file["lsp"]["zig"];
        assert_eq!(zig["command"], serde_norway::Value::from(command));
        assert_eq!(
            zig["root_markers"],
            serde_norway::Value::Sequence(vec![".git".into()])
        );
        assert!(zig.get("custom").is_none(), "{file:?}");
    }
}
