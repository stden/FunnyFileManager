//! Purpose: Validated engine commands carry typed operations and an allocated job identity.
use super::published::JobId;
use crate::automation::catalog::Invocation;
use crossterm::event::Event;
use ratatui::layout::Size;

pub(super) struct Command {
    pub id: JobId,
    pub operation: Operation,
}

pub(super) enum Operation {
    Action(Invocation),
    Input(Vec<Event>),
    Resize(Size),
}
