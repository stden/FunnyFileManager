//! Purpose: Export only visible file facts; connection credentials never enter panel snapshots.
use crate::models::{Panel, PanelKind};
use serde_json::Value;

mod preview;
mod rows;
mod virtuals;

pub(super) fn describe(panel: &Panel) -> Value {
    match &panel.kind {
        PanelKind::Files => rows::local(panel),
        PanelKind::Remote(remote) => rows::remote(remote),
        PanelKind::Sessions(sessions) => virtuals::sessions(sessions),
        PanelKind::Tree(tree) => virtuals::tree(tree),
        PanelKind::QuickView(view) => preview::describe(view),
    }
}

#[cfg(test)]
mod tests;
