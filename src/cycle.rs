//! Purpose: stepping round a fixed list of choices.
//!
//! Every control that offers a closed set of answers needs the same two
//! operations — the next one and the previous one, wrapping at both ends — and
//! every hand-written copy is another place for the wrap to be off by one.
//! The ring of controls in a dialog is the same problem: a tab order is a list
//! of choices that happens to be about the cursor.

/// The item one step from `current` in `order`, wrapping at both ends.
///
/// An item that is not in `order` steps to the first. That is the only answer
/// that leaves the caller usable — the alternative is a panic inside a dialog,
/// where the user has no way out.
#[must_use]
pub fn step<T: Copy + PartialEq>(order: &[T], current: T, forward: bool) -> T {
    let Some(first) = order.first().copied() else {
        return current;
    };
    let at = order.iter().position(|item| *item == current).unwrap_or(0);
    let next = if forward {
        at + 1
    } else {
        at + order.len() - 1
    };
    order.get(next % order.len()).copied().unwrap_or(first)
}

#[cfg(test)]
mod tests;
