//! Purpose: the Left and Right drop-downs — the ten view modes, then the panel's tools and settings.

use crate::action::{Action, PanelCommand};
use crate::menu::Entry;
use crate::models::{PanelSide, PanelView};

/// The Left or Right menu: `$toggle` is its Panel On/Off key, `$drive` its
/// Change drive key. A macro rather than a function, so the two tables
/// share their rows without one function running past the line limit.
macro_rules! panel_menu {
    ($side:expr, $toggle:literal, $drive:literal) => {
        [
            Entry::item("&Brief", "Ctrl+1", Action::PanelView(PanelView::Brief)),
            Entry::item("&Medium", "Ctrl+2", Action::PanelView(PanelView::Medium)),
            Entry::item("&Full", "Ctrl+3", Action::PanelView(PanelView::Full)),
            Entry::item("&Wide", "Ctrl+4", Action::PanelView(PanelView::Wide)),
            Entry::item(
                "Detai&led",
                "Ctrl+5",
                Action::PanelView(PanelView::Detailed),
            ),
            Entry::item(
                "&Descriptions",
                "Ctrl+6",
                Action::PanelView(PanelView::Descriptions),
            ),
            Entry::item(
                "Lon&g descriptions",
                "Ctrl+7",
                Action::PanelView(PanelView::LongDescriptions),
            ),
            Entry::item(
                "Fil&e owners",
                "Ctrl+8",
                Action::PanelView(PanelView::Owners),
            ),
            Entry::item("File lin&ks", "Ctrl+9", Action::PanelView(PanelView::Links)),
            Entry::item(
                "&Alternative full",
                "Ctrl+0",
                Action::PanelView(PanelView::Alternative),
            ),
            Entry::Separator,
            Entry::item("&Info panel", "Ctrl+L", Action::Information),
            Entry::item("&Tree panel", "Ctrl+T", Action::Panel(PanelCommand::Tree)),
            Entry::item(
                "Quick &view",
                "Ctrl+Q",
                Action::Panel(PanelCommand::QuickView),
            ),
            Entry::Separator,
            Entry::item(
                "&Sort modes",
                "Ctrl+F12",
                Action::Panel(PanelCommand::SortMenu),
            ),
            Entry::item(
                "Show lo&ng names",
                "Ctrl+N",
                Action::Panel(PanelCommand::LongNames),
            ),
            Entry::item(
                "Panel &On/Off",
                $toggle,
                Action::Panel(PanelCommand::TogglePanel($side)),
            ),
            Entry::item("&Re-read", "Ctrl+R", Action::Refresh),
            Entry::item("&Change drive", $drive, Action::DriveMenu($side)),
        ]
    };
}

/// A panel's own menu, row for row as Far draws it.
///
/// Ten view modes followed by panel tools and settings.
/// The renderer checks the active mode and current panel options.
pub(in crate::menu) const PANEL: [Entry; 20] = panel_menu!(PanelSide::Left, "Ctrl+F1", "Alt+F1");

/// The same, aimed at the right panel: only its toggle and drive keys differ —
/// Far gives each side its own.
pub(in crate::menu) const PANEL_RIGHT: [Entry; 20] =
    panel_menu!(PanelSide::Right, "Ctrl+F2", "Alt+F2");
