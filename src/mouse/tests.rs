//! Unit tests: the arithmetic that decides what a click landed on.

use super::*;

#[cfg(test)]
mod tests_bounds;
#[cfg(test)]
mod tests_clicks;
#[cfg(test)]
mod tests_dialog;
#[cfg(test)]
mod tests_fkeys;
#[cfg(test)]
mod tests_regions;

const KEYS: usize = 12;

/// `hit` on the default screen with both panels drawn.
fn both(column: u16, row: u16) -> Target {
    hit(SCREEN, column, row, KEYS, PanelsShown::Both)
}

// ─── the screen ─────────────────────────────────────────────────────────────

const SCREEN: Rect = Rect {
    x: 0,
    y: 0,
    width: 80,
    height: 24,
};

impl Target {
    fn is_row(self) -> bool {
        matches!(self, Self::PanelRow { .. })
    }

    fn side(self) -> Option<PanelSide> {
        match self {
            Self::PanelRow { side, .. } | Self::Panel(side) => Some(side),
            _ => None,
        }
    }
}
