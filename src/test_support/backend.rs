//! Purpose: Keep the public test adapter on the production memory-terminal implementation.
pub use crate::terminal::memory::MemoryBackend as WritableTestBackend;

#[cfg(test)]
mod tests;
