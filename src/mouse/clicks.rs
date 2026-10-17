//! Purpose: Panel double-click recognition — when a second press on the same entry activates it.

use std::time::{Duration, Instant};

use crossterm::event::MouseEventKind;

use crate::models::PanelSide;

/// A turn of the wheel, which every screen answers as a cursor move of its
/// own: a panel's, the console's, the viewer's, or a dialog's Up and Down.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Wheel {
    Up,
    Down,
}

impl Wheel {
    /// The turn `kind` reports, if it is one.
    #[must_use]
    pub const fn of(kind: MouseEventKind) -> Option<Self> {
        match kind {
            MouseEventKind::ScrollUp => Some(Self::Up),
            MouseEventKind::ScrollDown => Some(Self::Down),
            _ => None,
        }
    }
}

/// What a press on a panel row means for this click sequence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelRowClick {
    /// The first press selects the row.
    Select,
    /// A timely second press on the same entry performs Enter.
    Activate,
}

/// The two valid states of panel-row double-click recognition.
#[derive(Debug, Default)]
enum PanelClickState {
    /// No row is waiting for its second press.
    #[default]
    Idle,
    /// One row was pressed and may still become a double-click.
    Armed {
        side: PanelSide,
        index: usize,
        at: Instant,
    },
}

/// Recognises a double-click without putting timing flags into [`crate::models::App`].
///
/// Entry indices are tracked instead of screen rows. That keeps a click tied
/// to the entry the user selected even if rendering adjusts the list offset
/// before the second press arrives.
#[derive(Debug, Default)]
pub struct PanelClicks {
    state: PanelClickState,
}

impl PanelClicks {
    /// Register one left-button press on an existing panel entry.
    pub(crate) fn press(&mut self, side: PanelSide, index: usize, now: Instant) -> PanelRowClick {
        const DOUBLE_CLICK_INTERVAL: Duration = Duration::from_millis(500);

        let activates = match self.state {
            PanelClickState::Armed {
                side: previous_side,
                index: previous_index,
                at,
            } if previous_side == side && previous_index == index => now
                .checked_duration_since(at)
                .is_some_and(|elapsed| elapsed <= DOUBLE_CLICK_INTERVAL),
            PanelClickState::Idle | PanelClickState::Armed { .. } => false,
        };

        if activates {
            // A third press starts a new sequence instead of activating twice.
            self.state = PanelClickState::Idle;
            PanelRowClick::Activate
        } else {
            self.state = PanelClickState::Armed {
                side,
                index,
                at: now,
            };
            PanelRowClick::Select
        }
    }

    /// Break a pending panel double-click after an unrelated action.
    pub(crate) fn reset(&mut self) {
        self.state = PanelClickState::Idle;
    }
}
