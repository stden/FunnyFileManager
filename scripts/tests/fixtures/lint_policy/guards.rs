//! Negative: a leak, a shared pointer cloned like a value, an empty destructor,
//! and an allocation spelled like a conversion.

use std::rc::Rc;

/// Holds nothing to release.
pub struct Guard;

impl Drop for Guard {
    fn drop(&mut self) {}
}

/// Never frees the copy.
pub fn leak(text: &str) {
    std::mem::forget(text.to_owned());
}

/// Hides that only a pointer is copied.
#[must_use]
pub fn share(shared: &Rc<u8>) -> Rc<u8> {
    shared.clone()
}

/// Reads like a formatting step.
#[must_use]
pub fn owned() -> String {
    "name".to_string()
}
