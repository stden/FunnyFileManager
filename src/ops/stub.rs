//! Purpose: What a menu row that has no dialog yet says when chosen.
//!
//! A row must never silently do nothing — the user cannot tell a key that
//! was ignored from one that did something invisible. The status bar names
//! the feature and says it is on its way; when the last one lands, this goes.

use crate::models::App;
use crate::status::Status;

pub(super) fn report_coming(app: &mut App, feature: &str) {
    app.set_status(Status::CommandOutput(format!(
        "{feature}: coming in this release"
    )));
}
