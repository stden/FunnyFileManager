//! Purpose: Temporary-file planner and guarded file-only execution adapters.
mod policy;
mod remove;
mod scan;
pub use remove::remove;
pub use scan::scan;
#[cfg(test)]
mod tests;
