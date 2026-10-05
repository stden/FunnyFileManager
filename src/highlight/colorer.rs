//! Purpose: Real Colorer highlighting — the engine Far's `FarColorer` plugin uses.
//!
//! Colorer parses with the same HRC schemes Far ships, so a file looks here as
//! it looks there, for all 367 languages rather than the dozen the built-in
//! tokenizer knows. What comes back is a stream of *regions* —
//! `def:Keyword`, `python:pyString` — which [`region_token`] maps onto the
//! token classes the editor already paints.
//!
//! Optional, and off unless the `colorer` feature is on: the engine is C++ and
//! needs ICU, libxml2 and zlib, which is a large thing to make every build of a
//! file manager depend on. When it is absent, or when no catalog can be found,
//! [`Colorer::discover`] returns `None` and the built-in tokenizer does the
//! work — a missing scheme database must degrade to plainer colour, never to a
//! file that will not open.

use std::ffi::{c_void, CStr, CString};
use std::os::raw::{c_char, c_int};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

mod catalog;
mod token;

pub use token::{region_token, Span};

/// Serialises every call into Colorer, process-wide.
///
/// Not merely per-instance: Colorer reads its schemes through libxml2, whose
/// `xmlSetExternalEntityLoader` and `xmlSetGenericErrorFunc` are *global*. Two
/// threads building parsers at once race on those and segfault — which is what
/// the test suite did, in parallel, while each individual test passed.
static FFI: Mutex<()> = Mutex::new(());

/// A loaded Colorer scheme database.
pub struct Colorer {
    handle: Handle,
    catalog: PathBuf,
}

/// The raw pointer, wrapped so its `Send` claim is stated in one place.
struct Handle(*mut c_void);

// SAFETY: the pointer is only ever dereferenced while `FFI` is held, so at most
// one thread is inside Colorer — or inside the libxml2 globals it sets — at a
// time.
#[allow(unsafe_code, reason = "Send for the raw handle; every use holds `FFI`")]
unsafe impl Send for Handle {}

// SAFETY: as for `Send` — every use goes through `FFI`. `Sync` is needed
// because `SHARED` hands out `&'static Colorer`.
#[allow(unsafe_code, reason = "Sync for the raw handle; every use holds `FFI`")]
unsafe impl Sync for Handle {}

impl Drop for Colorer {
    #[allow(unsafe_code, reason = "frees the handle through the Colorer C API")]
    fn drop(&mut self) {
        let _guard = FFI.lock();
        // SAFETY: `self.handle.0` came from `colorer_open` and is freed once,
        // here, because `Colorer` owns it and is not `Clone`.
        unsafe { colorer_sys::colorer_close(self.handle.0) };
    }
}

impl Colorer {
    /// Load a specific catalog.
    #[must_use]
    #[allow(unsafe_code, reason = "opens a catalog through the Colorer C API")]
    pub fn open(catalog: &Path) -> Option<Self> {
        let path = CString::new(catalog.as_os_str().as_encoded_bytes()).ok()?;
        let handle = {
            let _guard = FFI.lock().ok()?;
            // SAFETY: `path` is a NUL-terminated string that outlives the call.
            unsafe { colorer_sys::colorer_open(path.as_ptr()) }
        };
        (!handle.is_null()).then(|| Self {
            handle: Handle(handle),
            catalog: catalog.to_path_buf(),
        })
    }

    /// The language Colorer chooses for `path` — `rust`, `python` — or `None`
    /// when it has no scheme for it.
    #[must_use]
    #[allow(unsafe_code, reason = "asks the Colorer C API for a file type")]
    pub fn file_type(&self, path: &Path) -> Option<String> {
        let name = CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
        let _guard = FFI.lock().ok()?;
        // SAFETY: the handle is live for as long as `self`, and `name` outlives
        // the call. The returned pointer is owned by the shim and copied here
        // before the lock is released, so no other call can overwrite it first.
        let raw = unsafe { colorer_sys::colorer_file_type(self.handle.0, name.as_ptr()) };
        if raw.is_null() {
            return None;
        }
        // SAFETY: non-null, and the shim returns a NUL-terminated name that
        // stays valid until the next call, which `FFI` holds off.
        let chosen = unsafe { CStr::from_ptr(raw) }
            .to_string_lossy()
            .into_owned();
        // Colorer answers `default` when it recognised nothing, which is a "no"
        // wearing a name.
        (chosen != "default").then_some(chosen)
    }

    /// Regions for every line of `lines`, indexed by line.
    ///
    /// Colorer reports overlapping regions — a bracket is both `def:Symbol` and
    /// `def:PairStart` — so a line's entry is a list, and the *last* region to
    /// cover a character is the one that wins. That is Colorer's own rule and
    /// the reason the editor cannot simply take the first.
    #[must_use]
    #[allow(unsafe_code, reason = "parses lines through the Colorer C API")]
    pub fn parse(&self, path: &Path, lines: &[String]) -> Vec<Vec<Span>> {
        let mut collected: Vec<Vec<Span>> = vec![Vec::new(); lines.len()];
        let Ok(name) = CString::new(path.as_os_str().as_encoded_bytes()) else {
            return collected;
        };
        // The shim reads `const char*`, so every line needs a NUL. A line with
        // an interior NUL cannot be passed, and is sent as empty rather than
        // truncating the file at that point.
        let owned: Vec<CString> = lines
            .iter()
            .map(|line| CString::new(line.as_str()).unwrap_or_default())
            .collect();
        let pointers: Vec<*const c_char> = owned.iter().map(|line| line.as_ptr()).collect();

        let Ok(_guard) = FFI.lock() else {
            return collected;
        };
        // SAFETY: `pointers` and the `CString`s it points into outlive the
        // call; `collect_region` only writes through the `&mut Vec` handed to
        // it as `ctx`, which is borrowed for exactly this call.
        unsafe {
            colorer_sys::colorer_parse(
                self.handle.0,
                name.as_ptr(),
                pointers.as_ptr(),
                pointers.len(),
                Some(collect_region),
                std::ptr::addr_of_mut!(collected).cast::<c_void>(),
            );
        }
        collected
    }
}

/// The C callback. Anything it cannot make sense of is dropped rather than
/// guessed at — a region on a line that does not exist is a bug in the shim,
/// not something to paint.
///
/// # Safety
///
/// `ctx` must be the `&mut Vec<Vec<Span>>` handed to `colorer_parse`, with no
/// other borrow alive, and `region` null or a NUL-terminated string; Colorer
/// calls it only synchronously from inside that `colorer_parse` call.
#[allow(unsafe_code, reason = "the callback Colorer calls with raw pointers")]
unsafe extern "C" fn collect_region(
    ctx: *mut c_void,
    line: usize,
    sx: c_int,
    ex: c_int,
    region: *const c_char,
) {
    if ctx.is_null() || region.is_null() || sx < 0 || ex <= sx {
        return;
    }
    // SAFETY: `ctx` is the `&mut Vec<Vec<Span>>` passed to `colorer_parse`, and
    // Colorer calls this synchronously from inside that call.
    let collected = unsafe { &mut *ctx.cast::<Vec<Vec<Span>>>() };
    let Some(row) = collected.get_mut(line) else {
        return;
    };
    // SAFETY: the shim passes a NUL-terminated name that outlives the call.
    let name = unsafe { CStr::from_ptr(region) };
    let Ok(name) = name.to_str() else { return };
    // Guarded above (`sx < 0 || ex <= sx`), but `try_from` says so where the
    // conversion happens rather than twenty lines away.
    let (Ok(start), Ok(end)) = (usize::try_from(sx), usize::try_from(ex)) else {
        return;
    };
    row.push(Span {
        start,
        end,
        token: region_token(name),
    });
}

#[cfg(test)]
mod tests;
