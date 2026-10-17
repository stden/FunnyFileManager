//! Show the timing report using the normal scrolling viewer.

use crate::models::{App, Overlay, Viewer};

pub fn open(app: &mut App) {
    app.set_overlay(Overlay::Viewer(Viewer::report(
        "Action statistics",
        app.performance.report(),
    )));
}

#[cfg(test)]
mod tests;
