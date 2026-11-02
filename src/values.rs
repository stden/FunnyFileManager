//! Purpose: Plain values that every layer may use.
//!
//! A directory entry, an image, a remote path and a saved connection. None of
//! them depends on application state, so adapters may use them directly.

pub mod entry;
pub mod image;
pub mod remote_path;
pub mod session;
