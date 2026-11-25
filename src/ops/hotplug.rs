//! Purpose: Hotplug devices list: removable devices, where the platform can list them.

use crate::models::{App, Overlay};

/// the Commands menu: open the Hotplug devices list dialog.
pub fn open(app: &mut App) {
    app.set_overlay(Overlay::Hotplug(Box::default()));
}
