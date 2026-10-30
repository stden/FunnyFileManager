//! Purpose: The suggestion list open under a form field, passive until the
//! arrows reach into it.

use crate::models::Menu;

use super::Direction;

/// Where the keyboard is while a suggestion list is open. A passive list is
/// shown, but the field keeps Enter; an active one has a highlighted row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListFocus {
    Field,
    List,
}

/// The suggestion list open under a field. `Menu` cannot be empty, so a list
/// with nothing to highlight is a list whose focus is still on the field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suggestions {
    menu: Menu,
    focus: ListFocus,
}

impl Suggestions {
    /// A list opened by typing: nothing highlighted, Enter still presses OK.
    #[must_use]
    pub const fn passive(menu: Menu) -> Self {
        Self {
            menu,
            focus: ListFocus::Field,
        }
    }

    /// A list opened on request, its first row highlighted.
    #[must_use]
    pub const fn active(menu: Menu) -> Self {
        Self {
            menu,
            focus: ListFocus::List,
        }
    }

    #[must_use]
    pub const fn menu(&self) -> &Menu {
        &self.menu
    }

    #[must_use]
    pub const fn focus(&self) -> ListFocus {
        self.focus
    }

    /// Down / Up: a passive list becomes active on its first / last row, an
    /// active one moves its highlight, wrapping.
    pub fn step(&mut self, direction: Direction) {
        match (self.focus, direction) {
            (ListFocus::Field, Direction::Next) => self.focus = ListFocus::List,
            (ListFocus::Field, Direction::Previous) => {
                self.focus = ListFocus::List;
                self.menu.previous();
            }
            (ListFocus::List, Direction::Next) => self.menu.next(),
            (ListFocus::List, Direction::Previous) => self.menu.previous(),
        }
    }

    /// The list refiltered to `menu`, keeping its focus; an active list's
    /// highlight returns to the first row.
    #[must_use]
    pub const fn refilled(&self, menu: Menu) -> Self {
        Self {
            menu,
            focus: self.focus,
        }
    }
}
