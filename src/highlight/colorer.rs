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
use std::sync::{Mutex, OnceLock};

use super::Token;

/// Where a catalog is looked for, in order, when `COLORER_CATALOG` is unset.
///
/// The first is where `FarColorer` keeps it on a Far install; the rest are where
/// a hand-built scheme checkout tends to land.
const CATALOG_CANDIDATES: [&str; 3] = [
    "/usr/share/colorer/catalog.xml",
    "/usr/local/share/colorer/catalog.xml",
    "/opt/colorer/catalog.xml",
];

/// The same, relative to the user's home — where `ffm.sh colorer` and
/// `ffm.cmd colorer` install the schemes, since that needs no root.
const HOME_CANDIDATES: [&str; 2] = [
    ".local/share/ffm/colorer/catalog.xml",
    ".local/share/ffm/Colorer-schemes/base/catalog.base.xml",
];

/// Serialises every call into Colorer, process-wide.
///
/// Not merely per-instance: Colorer reads its schemes through libxml2, whose
/// `xmlSetExternalEntityLoader` and `xmlSetGenericErrorFunc` are *global*. Two
/// threads building parsers at once race on those and segfault — which is what
/// the test suite did, in parallel, while each individual test passed.
static FFI: Mutex<()> = Mutex::new(());

/// The one scheme database this process loads.
static SHARED: OnceLock<Option<Colorer>> = OnceLock::new();

/// A loaded Colorer scheme database.
pub struct Colorer {
    handle: Handle,
    catalog: PathBuf,
}

/// The raw pointer, wrapped so its `Send` claim is stated in one place.
struct Handle(*mut c_void);

// SAFETY: the pointer is only ever dereferenced while `FFI` is held, so at most
// one thread is inside Colorer — or inside the libxml2 globals it sets — at a
// time. `Sync` as well as `Send`, because `SHARED` hands out `&'static Colorer`.
unsafe impl Send for Handle {}
unsafe impl Sync for Handle {}

impl Drop for Colorer {
    fn drop(&mut self) {
        let _guard = FFI.lock();
        // SAFETY: `self.handle.0` came from `colorer_open` and is freed once,
        // here, because `Colorer` owns it and is not `Clone`.
        unsafe { colorer_sys::colorer_close(self.handle.0) };
    }
}

impl Colorer {
    /// Load the catalog named by `COLORER_CATALOG`, or the first of
    /// [`CATALOG_CANDIDATES`] that exists.
    ///
    /// `None` rather than an error: no scheme database is an ordinary state,
    /// not a failure, and the caller's answer to it is the same either way.
    /// The process's scheme database, loaded once.
    ///
    /// Shared rather than per-editor: `catalog.xml` pulls in hundreds of
    /// schemes, and re-reading them every time a file is opened would be felt.
    #[must_use]
    pub fn shared() -> Option<&'static Self> {
        SHARED.get_or_init(Self::discover).as_ref()
    }

    #[must_use]
    fn discover() -> Option<Self> {
        let catalog = std::env::var_os("COLORER_CATALOG")
            .map(PathBuf::from)
            .filter(|path| path.is_file())
            .or_else(|| {
                CATALOG_CANDIDATES
                    .iter()
                    .map(PathBuf::from)
                    .find(|path| path.is_file())
            })
            .or_else(|| {
                // Windows has no HOME; USERPROFILE is the same directory, and
                // the installer scripts put the schemes under the same tail.
                let home = std::env::var_os("HOME")
                    .or_else(|| std::env::var_os("USERPROFILE"))
                    .map(PathBuf::from)?;
                HOME_CANDIDATES
                    .iter()
                    .map(|tail| home.join(tail))
                    .find(|path| path.is_file())
            })?;
        Self::open(&catalog)
    }

    /// Load a specific catalog.
    #[must_use]
    pub fn open(catalog: &Path) -> Option<Self> {
        let path = CString::new(catalog.as_os_str().as_encoded_bytes()).ok()?;
        // SAFETY: `path` is a NUL-terminated string that outlives the call.
        let handle = {
            let _guard = FFI.lock().ok()?;
            unsafe { colorer_sys::colorer_open(path.as_ptr()) }
        };
        (!handle.is_null()).then(|| Self {
            handle: Handle(handle),
            catalog: catalog.to_path_buf(),
        })
    }

    /// The catalog this was loaded from, for reporting.
    #[must_use]
    pub fn catalog(&self) -> &Path {
        &self.catalog
    }

    /// The language Colorer chooses for `path` — `rust`, `python` — or `None`
    /// when it has no scheme for it.
    #[must_use]
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

/// One region Colorer found: a half-open character range and what it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub token: Token,
}

/// The C callback. Anything it cannot make sense of is dropped rather than
/// guessed at — a region on a line that does not exist is a bug in the shim,
/// not something to paint.
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

/// Map one of Colorer's region names onto a token class.
///
/// Colorer's vocabulary is far larger than the editor's palette — every
/// language adds its own `python:pyKeyword`, `rust:rustMacro` — but the names
/// are systematic: a language region is conventionally the `def:` one with a
/// prefix. Matching on the *suffix* therefore covers languages this code has
/// never heard of, which is the point of using Colorer at all.
#[must_use]
pub fn region_token(region: &str) -> Token {
    // `python:pyKeyword` → `pyKeyword`; `def:Keyword` → `Keyword`.
    let tail = region.rsplit(':').next().unwrap_or(region);
    let lower = tail.to_ascii_lowercase();

    // Order matters: `CommentContent` must be a comment before `Content`
    // matches anything else, and `StringEdge` a string before `Edge` does.
    for (needle, token) in [
        ("comment", Token::Comment),
        ("string", Token::Str),
        ("character", Token::Str),
        ("number", Token::Number),
        ("keyword", Token::Keyword),
        ("word", Token::Keyword),
        ("directive", Token::Meta),
        ("preprocessor", Token::Meta),
        ("outline", Token::Meta),
        ("type", Token::Type),
        ("class", Token::Type),
        ("function", Token::Type),
        ("symbol", Token::Punctuation),
        ("operator", Token::Punctuation),
        ("delimiter", Token::Punctuation),
        ("pair", Token::Punctuation),
    ] {
        if lower.contains(needle) {
            return token;
        }
    }
    Token::Normal
}

#[cfg(test)]
mod tests;
