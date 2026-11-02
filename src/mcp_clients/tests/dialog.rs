//! Purpose: The lines the dialog and `:mcp` draw: a client's row label and
//! the server's lines.

use super::*;
use crate::status::PathMessage;

#[test]
fn a_row_label_lines_up_name_presence_and_state() {
    let status = ClientStatus {
        client: ClientId::Codex,
        presence: Presence::Detected,
        config: None,
        state: ConfigState::DifferentEntry,
    };
    assert_eq!(
        row_label(&status),
        "Codex        detected      ≠ different entry"
    );
}

#[test]
fn server_lines_name_the_server_or_say_how_to_install_it() {
    let found = ServerState::Found(server());
    assert_eq!(
        server_lines(&found),
        [PathMessage::quoting("Server: ", server().as_str(), "")]
    );
    let missing: Vec<String> = server_lines(&ServerState::Missing)
        .iter()
        .map(ToString::to_string)
        .collect();
    assert_eq!(
        missing,
        [
            "Server: ffm-mcp was not found next to ffm or on PATH.",
            "Install it from the FFM sources: cargo install --path mcp",
        ]
    );
    let odd_path = absolute("/srv/x/odd");
    let odd = ServerState::NotUnicode(odd_path.clone());
    assert_eq!(
        server_lines(&odd),
        [
            PathMessage::quoting(
                "Server: ",
                odd_path.to_string_lossy(),
                " is not valid Unicode, so no config can name it."
            ),
            PathMessage::from(INSTALL_HINT.to_owned()),
        ]
    );
}
