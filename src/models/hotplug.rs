//! Purpose: The hotplug devices list: removable devices and their mount points.
//!
//! State only: the operations that fill it live in `ops`, and the keys that
//! move through it in `popup`.

/// The devices on screen and where the cursor is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DeviceList {
    pub cursor: usize,
}
