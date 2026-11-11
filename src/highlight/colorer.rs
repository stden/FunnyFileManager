//! Purpose: Real Colorer highlighting — the engine Far's `FarColorer` plugin uses.
//! Optional (`colorer` feature): maps HRC regions via [`region_token`]; missing catalogs return `None`.

use std::{
    ffi::{c_void, CStr, CString},
    os::raw::{c_char, c_int},
    path::{Path, PathBuf},
    sync::{Mutex, PoisonError},
};

mod catalog;
mod parse;
mod token;

pub use token::{region_token, Span};

/// Serialises every call into Colorer, process-wide, including libxml2 globals.
static FFI: Mutex<()> = Mutex::new(());

type Raw = *mut c_void;
type Str = *const c_char;
type ParseCb = unsafe extern "C" fn(Raw, usize, c_int, c_int, Str);
type ParseFn = unsafe extern "C" fn(Raw, Str, *const Str, usize, Option<ParseCb>, Raw) -> c_int;

#[derive(Clone, Copy)]
struct Api {
    open: unsafe extern "C" fn(Str) -> Raw,
    close: unsafe extern "C" fn(Raw),
    file_type: unsafe extern "C" fn(Raw, Str) -> Str,
    parse: ParseFn,
}

const DEFAULT_API: Api = Api {
    open: colorer_sys::colorer_open,
    close: colorer_sys::colorer_close,
    file_type: colorer_sys::colorer_file_type,
    parse: colorer_sys::colorer_parse,
};

#[cfg(test)]
thread_local! {
    static TEST_API: std::cell::Cell<Option<Api>> = const { std::cell::Cell::new(None) };
}

fn api() -> Api {
    #[cfg(test)]
    if let Some(custom) = TEST_API.with(std::cell::Cell::get) {
        return custom;
    }
    DEFAULT_API
}

/// A loaded Colorer scheme database.
pub struct Colorer {
    handle: Handle,
    catalog: PathBuf,
}

struct Handle(*mut c_void);

// SAFETY: only ever dereferenced while `FFI` is held.
#[allow(unsafe_code, reason = "Send/Sync for raw handle guarded by `FFI`")]
unsafe impl Send for Handle {}
// SAFETY: every use goes through `FFI`.
#[allow(unsafe_code, reason = "Send/Sync for raw handle guarded by `FFI`")]
unsafe impl Sync for Handle {}

impl Drop for Colorer {
    #[allow(unsafe_code, reason = "frees the handle through the Colorer C API")]
    fn drop(&mut self) {
        let _guard = FFI.lock();
        // SAFETY: `self.handle.0` came from `colorer_open` and is freed once.
        unsafe { (api().close)(self.handle.0) };
    }
}

impl Colorer {
    /// Load a specific catalog.
    #[must_use]
    #[allow(unsafe_code, reason = "opens a catalog through the Colorer C API")]
    pub fn open(catalog: &Path) -> Option<Self> {
        let path = CString::new(catalog.as_os_str().as_encoded_bytes()).ok()?;
        let _guard = FFI.lock().ok()?;
        // SAFETY: `path` is NUL-terminated and outlives the call.
        let handle = unsafe { (api().open)(path.as_ptr()) };
        (!handle.is_null()).then(|| Self {
            handle: Handle(handle),
            catalog: catalog.to_path_buf(),
        })
    }

    /// The language Colorer chooses for `path`, or `None` when unknown.
    #[must_use]
    #[allow(unsafe_code, reason = "asks the Colorer C API for a file type")]
    pub fn file_type(&self, path: &Path) -> Option<String> {
        let name = CString::new(path.as_os_str().as_encoded_bytes()).ok()?;
        let _guard = FFI.lock().ok()?;
        // SAFETY: live handle and `name`; returned string copied under `FFI`.
        let raw = unsafe { (api().file_type)(self.handle.0, name.as_ptr()) };
        (!raw.is_null())
            // SAFETY: non-null NUL-terminated shim string valid under `FFI`.
            .then(|| parse::non_default_type(unsafe { CStr::from_ptr(raw) }))
            .flatten()
    }

    /// Regions for every line of `lines`, indexed by line.
    #[must_use]
    #[allow(unsafe_code, reason = "parses lines through the Colorer C API")]
    pub fn parse(&self, path: &Path, lines: &[String]) -> Vec<Vec<Span>> {
        let mut collected: Vec<Vec<Span>> = vec![Vec::new(); lines.len()];
        let Some((name, _owned, ptrs)) = parse::encode_inputs(path, lines) else {
            return collected;
        };
        let _guard = FFI.lock().unwrap_or_else(PoisonError::into_inner);
        let ctx = std::ptr::addr_of_mut!(collected).cast::<c_void>();
        // SAFETY: `ptrs` and `_owned` outlive the call; `ctx` borrows `collected`.
        unsafe {
            let _ = (api().parse)(
                self.handle.0,
                name.as_ptr(),
                ptrs.as_ptr(),
                ptrs.len(),
                Some(collect_region),
                ctx,
            );
        }
        parse::utf16_spans_to_chars(lines, &mut collected);
        collected
    }
}

/// The C callback: forwards raw pointers into [`parse::push_region`].
#[allow(unsafe_code, reason = "the callback Colorer calls with raw pointers")]
unsafe extern "C" fn collect_region(ctx: Raw, line: usize, sx: c_int, ex: c_int, region: Str) {
    if ctx.is_null() || region.is_null() {
        return;
    }
    // SAFETY: `ctx` points to the `Vec<Vec<Span>>` borrowed for this `parse` call.
    let collected = unsafe { &mut *ctx.cast::<Vec<Vec<Span>>>() };
    // SAFETY: `region` is a non-null NUL-terminated string valid for the callback.
    let name = unsafe { CStr::from_ptr(region) };
    parse::push_region(collected, line, sx, ex, name);
}

#[cfg(test)]
#[allow(unsafe_code, reason = "fake C callbacks for unit-testing FFI paths")]
mod fake_ffi {
    use super::*;
    fn test_cstr<'a>(ptr: Str) -> &'a CStr {
        // SAFETY: test callers always pass valid NUL-terminated C strings.
        unsafe { CStr::from_ptr(ptr) }
    }
    unsafe extern "C" fn open(path: Str) -> Raw {
        tests_fake::open_ptr(test_cstr(path))
    }
    unsafe extern "C" fn close(_handle: Raw) {
        tests_fake::record_close();
    }
    unsafe extern "C" fn file_type(_h: Raw, path: Str) -> Str {
        tests_fake::classify_ptr(test_cstr(path))
    }
    unsafe extern "C" fn parse(
        _h: Raw,
        path: Str,
        lines: *const Str,
        count: usize,
        cb: Option<ParseCb>,
        ctx: Raw,
    ) -> c_int {
        let path = test_cstr(path);
        let first = (count > 0).then(|| {
            // SAFETY: `count > 0` guarantees `lines` points to at least one `Str`.
            test_cstr(unsafe { *lines })
        });
        if let Some(cb) = cb {
            for (c, line, sx, ex, reg) in tests_fake::events_for(path, first, ctx) {
                // SAFETY: invokes `collect_region` with test-controlled arguments.
                unsafe { cb(c, line, sx, ex, reg) };
            }
        }
        tests_fake::parse_rc(path)
    }
    pub(super) fn parse_without_cb() -> c_int {
        let null = std::ptr::null_mut();
        // SAFETY: `count == 0` and `cb == None` only read the static NUL-terminated path.
        unsafe { parse(null, c"x".as_ptr(), std::ptr::null(), 0, None, null) }
    }
    pub(super) const API: Api = Api {
        open,
        close,
        file_type,
        parse,
    };
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_fake;
