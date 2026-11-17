//! Unit tests: the Install MCP server dialog draws paths as they are — an
//! ampersand is text, not a hotkey mark — and a deep server path keeps the
//! program's name.

use super::tests::{draw, machine};
use crate::mcp_clients::{Host, SERVER_NAME};
use crate::models::{App, Panel};
use crate::test_support::{framed_row, last_component};

/// A folder name long enough that no row of an 80-column dialog holds a
/// path through it whole, whatever the temp directory's own length.
const DEEP: &str = "a-folder-name-long-enough-that-no-dialog-row-holds-the-whole-path";

/// Review finding: `&` in a path was taken for a hotkey marker, so
/// `/srv/x/R&D` was drawn as `/srv/x/RD` with the `D` lit up.
///
/// The folder sits deeper than the row is wide. On the Windows runner the
/// temp directory alone made the row lose its end, `R&D` with it: a path is
/// cut from its head, and the folder's own name is what stays.
#[test]
fn an_ampersand_in_a_path_is_drawn_as_it_is() {
    let machine = machine();
    machine.install_server().unwrap();
    let folder = machine.folder().join(DEEP).join("R&D");
    std::fs::create_dir_all(&folder).unwrap();
    let mut app = App::with_panels(
        Panel::new(folder.clone()).unwrap(),
        Panel::new(folder).unwrap(),
    );
    app.live.mcp_host = machine.host();
    crate::ops::mcp_install::open(&mut app);
    let (screen, _) = draw(&mut app);
    let row = framed_row(&screen, "Current folder: ");
    assert!(row.starts_with("Current folder: …"), "{row:?}\n{screen}");
    assert!(row.ends_with(&last_component("R&D")), "{row:?}\n{screen}");
}

/// A server installed deep in the tree is still named: the row keeps
/// `ffm-mcp` and gives up the head of its folder.
#[test]
fn a_deep_server_path_keeps_the_program_name() {
    let machine = machine();
    let beside_ffm = machine.exe_dir().join(DEEP);
    std::fs::create_dir(&beside_ffm).unwrap();
    crate::test_support::write_executable(&beside_ffm.join(SERVER_NAME), b"fake").unwrap();
    let mut app = App::with_panels(
        Panel::new(machine.folder()).unwrap(),
        Panel::new(machine.folder()).unwrap(),
    );
    app.live.mcp_host = Host {
        exe_dir: Some(beside_ffm),
        ..machine.host()
    };
    crate::ops::mcp_install::open(&mut app);
    let (screen, _) = draw(&mut app);
    let row = framed_row(&screen, "Server: ");
    assert!(row.starts_with("Server: …"), "{row:?}\n{screen}");
    assert!(
        row.ends_with(&last_component(SERVER_NAME)),
        "{row:?}\n{screen}"
    );
}
