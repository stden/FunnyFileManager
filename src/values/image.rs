//! Image pixels and asynchronous viewer state, independent of I/O.

use std::sync::Arc;

/// Decoded RGBA pixels and details of their source file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedImage {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: &'static str,
    pub(crate) size: u64,
    pub(crate) pixels: Arc<[u8]>,
}

impl DecodedImage {
    #[must_use]
    pub const fn width(&self) -> u32 {
        self.width
    }
    #[must_use]
    pub const fn height(&self) -> u32 {
        self.height
    }
    #[must_use]
    pub const fn format(&self) -> &'static str {
        self.format
    }
    #[must_use]
    pub const fn size(&self) -> u64 {
        self.size
    }
}

/// The status of a bounded background image request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImageState {
    Pending,
    Loading(u64),
    Ready(DecodedImage),
    Failed(String),
}
