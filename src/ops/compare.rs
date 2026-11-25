//! Purpose: Compare folders (Commands menu) — mark, in both panels, every
//! file with no namesake on the other side or a different size or time.
//!
//! No dialog: the answer is the marks themselves, and the status bar gives
//! the count. The comparison arrives with the feature.

use crate::models::App;

pub fn run(app: &mut App) {
    super::stub::report_coming(app, "Compare folders");
}

#[cfg(test)]
mod tests;
