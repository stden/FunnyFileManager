//! Purpose: Raw FFI to Colorer-library — the engine behind Far's FarColorer.
//!
//! Nothing here is safe or idiomatic; it is the C surface `shim.cpp` exposes
//! and no more. The safe wrapper lives in the file manager, so that this crate
//! stays a description of the foreign library rather than a second opinion
//! about how to use it.

use std::os::raw::{c_char, c_int, c_void};

/// Called once per region the parser finds: the line, the half-open character
/// range within it, and Colorer's fully-qualified region name (`def:Keyword`,
/// `python:pyString`).
pub type RegionCallback = Option<
    unsafe extern "C" fn(
        ctx: *mut c_void,
        line: usize,
        sx: c_int,
        ex: c_int,
        region: *const c_char,
    ),
>;

/// Whether a Colorer-library was found at build time.
///
/// The stubs below keep the crate compiling — and every caller's code
/// unchanged — on a machine that has none.
#[must_use]
pub const fn available() -> bool {
    cfg!(colorer)
}

#[cfg(colorer)]
extern "C" {
    /// Load a `catalog.xml`. Returns null if it cannot be read.
    pub fn colorer_open(catalog_path: *const c_char) -> *mut c_void;

    pub fn colorer_close(handle: *mut c_void);

    /// Parse `count` UTF-8 lines, reporting every region through `cb`.
    /// Returns the number of lines parsed, or -1.
    pub fn colorer_parse(
        handle: *mut c_void,
        filename: *const c_char,
        lines: *const *const c_char,
        count: usize,
        cb: RegionCallback,
        ctx: *mut c_void,
    ) -> c_int;

    /// The Colorer file type chosen for `filename` (`rust`, `python`), or null.
    pub fn colorer_file_type(handle: *mut c_void, filename: *const c_char) -> *const c_char;
}

/// Stubs for a build with no Colorer-library. Every one answers "nothing here",
/// which is exactly what the safe wrapper already handles.
#[cfg(not(colorer))]
mod stub {
    use super::{c_char, c_int, c_void, RegionCallback};

    /// # Safety
    /// Never dereferences its argument.
    pub unsafe extern "C" fn colorer_open(_catalog_path: *const c_char) -> *mut c_void {
        std::ptr::null_mut()
    }

    /// # Safety
    /// Never dereferences its argument.
    pub unsafe extern "C" fn colorer_close(_handle: *mut c_void) {}

    /// # Safety
    /// Never dereferences its arguments.
    pub unsafe extern "C" fn colorer_parse(
        _handle: *mut c_void,
        _filename: *const c_char,
        _lines: *const *const c_char,
        _count: usize,
        _cb: RegionCallback,
        _ctx: *mut c_void,
    ) -> c_int {
        -1
    }

    /// # Safety
    /// Never dereferences its arguments.
    pub unsafe extern "C" fn colorer_file_type(
        _handle: *mut c_void,
        _filename: *const c_char,
    ) -> *const c_char {
        std::ptr::null()
    }
}

#[cfg(not(colorer))]
pub use stub::{colorer_close, colorer_file_type, colorer_open, colorer_parse};
