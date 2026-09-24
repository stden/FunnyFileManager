//! Negative: output that bypasses the UI, and an order a hash map does not keep.

use std::collections::HashMap;
use std::fmt;

/// Writes over the TUI's screen.
pub fn report(count: usize) {
    println!("{count} files");
    eprintln!("{count} files");
}

/// A size shown to the user.
pub struct Size(pub Option<u64>);

impl fmt::Display for Size {
    // Shows the user Rust's debug format: `Some(3)`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{:?}", self.0)
    }
}

/// Names in an order that changes from run to run.
#[must_use]
pub fn names() -> Vec<String> {
    let sizes = HashMap::from([("a".to_owned(), 1_u8), ("b".to_owned(), 2)]);
    let mut names = Vec::new();
    for name in sizes.keys() {
        names.push(name.clone());
    }
    names
}
