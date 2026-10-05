//! Purpose: The settings dialog engine's state — one dialog for every page
//! of Far's Options menu.
//!
//! One type for twenty-two pages, because the pages differ only in their
//! rows: the walking, the toggling and the OK/Cancel are the same, and a
//! dialog per page would be twenty-two copies of them.

use crate::menu::SettingsPage as OptionsPage;

/// Which page is open. The rows arrive with the feature.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SettingsDialog {
    pub page: OptionsPage,
}

impl SettingsDialog {
    #[must_use]
    pub const fn for_page(page: OptionsPage) -> Self {
        Self { page }
    }
}
