//! Purpose: Test seam: make the search's `read_dir` fail for one folder name,
//! so an unreadable folder can be staged without file permissions.

use std::path::Path;

thread_local! {
    static DENIED: std::cell::RefCell<Option<String>> = const { std::cell::RefCell::new(None) };
}

/// Fail `read_dir` for a directory whose last component is `name`.
pub(in crate::ops::menu) struct DenyReads {
    previous: Option<String>,
}

impl DenyReads {
    pub(in crate::ops::menu) fn named(name: &str) -> Self {
        let previous = DENIED.with(|slot| slot.borrow_mut().replace(name.to_owned()));
        Self { previous }
    }
}

impl Drop for DenyReads {
    fn drop(&mut self) {
        DENIED.with(|slot| *slot.borrow_mut() = self.previous.take());
    }
}

pub(super) fn denied(path: &Path) -> bool {
    DENIED.with(|slot| {
        slot.borrow()
            .as_ref()
            .is_some_and(|name| path.ends_with(name.as_str()))
    })
}
