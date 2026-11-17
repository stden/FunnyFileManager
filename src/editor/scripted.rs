//! Purpose: F4 without an editor process — each outcome an external editor can
//! have, reported as it would be, for the integration and BDD suites.

use std::io;

use crate::models::App;
use crate::status::Status;

#[derive(Clone, Copy, Debug)]
pub enum TestEditorBehavior {
    Success,
    FailLaunch,
    ExitError,
}

pub fn handle_edit_test_mode(app: &mut App, behavior: TestEditorBehavior) -> io::Result<()> {
    let Some(selected_entry) = app.get_active_panel().get_selected_entry() else {
        return Ok(());
    };

    if selected_entry.kind.is_dir() {
        app.set_status(Status::EditDirUnsupported);
        return Ok(());
    }

    let Some(selected) = app.get_active_panel().selected_path() else {
        return Ok(());
    };

    match behavior {
        TestEditorBehavior::FailLaunch => {
            app.set_status(Status::LaunchEditorFailed {
                error: "No such file or directory (os error 2)".to_owned(),
            });
        }
        TestEditorBehavior::ExitError => {
            app.set_status(Status::EditorExited {
                status: "exit status: 1".to_owned(),
            });
        }
        TestEditorBehavior::Success => {
            app.set_status(Status::Edited(selected.display().to_string()));
        }
    }

    app.left_panel.load_entries()?;
    app.left_panel.ensure_selection();
    app.right_panel.load_entries()?;
    app.right_panel.ensure_selection();
    Ok(())
}
