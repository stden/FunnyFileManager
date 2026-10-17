//! Purpose: a printable key types into the command line instead of acting.

use super::*;

/// Typing in the panels starts a command, the way Far Manager and every shell
/// do — the character is not swallowed, and it is not a quick search either
/// (that stayed on Alt+letter).
#[test]
fn a_printable_key_opens_the_command_line_holding_it() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    for ch in ['l', 's'] {
        dispatch_key(
            &mut terminal,
            &mut app,
            KeyEvent::new(KeyCode::Char(ch), KeyModifiers::empty()),
        );
    }

    assert_eq!(app.get_command_line(), Some("ls"));
    assert!(
        app.quick_search.query().is_none(),
        "typing is not a quick search"
    );
    Ok(())
}

/// `q` is the first letter of `quit`, `qemu` and `queue`. It has to be
/// typeable, so it starts a command instead of ending the program; F10 still
/// quits, and so does the `quit` command it can now spell.
#[test]
fn bare_q_types_instead_of_quitting() -> io::Result<()> {
    let temp_dir = TempDir::new().expect("test setup");
    let mut app = App::new().expect("test setup");
    app.left_panel = Panel::new(temp_dir.path().to_path_buf()).expect("test setup");

    let backend = WritableTestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).expect("test setup");

    dispatch_key(
        &mut terminal,
        &mut app,
        KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()),
    );

    assert!(app.lifecycle.is_running(), "q must not quit");
    assert_eq!(app.get_command_line(), Some("q"));
    assert!(app.lifecycle.is_running());
    Ok(())
}
