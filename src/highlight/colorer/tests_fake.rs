//! Unit tests exercising Colorer FFI, catalog discovery, highlighter, and cache
//! deterministically even on machines without native `Colorer-library`.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::OnceLock;

use super::*;
use crate::highlight::cache::HighlightCache;
use crate::highlight::{Highlighter, Token};

static CLOSES: AtomicUsize = AtomicUsize::new(0);
static FAKE_SHARED: OnceLock<Colorer> = OnceLock::new();
const BAD_UTF8: &[u8] = b"\xff\0";

pub(super) type FakeEvent = (Raw, usize, c_int, c_int, Str);

const fn span(start: usize, end: usize, token: Token) -> Span {
    Span { start, end, token }
}

pub(super) fn open_ptr(path: &CStr) -> *mut c_void {
    if path.to_string_lossy().ends_with("null.xml") {
        std::ptr::null_mut()
    } else {
        std::ptr::dangling_mut::<u8>().cast()
    }
}

pub(super) fn record_close() {
    CLOSES.fetch_add(1, Ordering::Relaxed);
}

pub(super) fn classify_ptr(path: &CStr) -> *const c_char {
    let text = path.to_string_lossy();
    if text.ends_with(".lua") {
        c"lua".as_ptr()
    } else if text.ends_with(".zzzz") {
        c"default".as_ptr()
    } else {
        std::ptr::null()
    }
}

pub(super) fn parse_rc(path: &CStr) -> c_int {
    if path.to_string_lossy().ends_with(".fail") {
        -1
    } else {
        0
    }
}

pub(super) fn events_for(path: &CStr, first: Option<&CStr>, ctx: *mut c_void) -> Vec<FakeEvent> {
    if path.to_string_lossy().ends_with(".fail") {
        return Vec::new();
    }
    let kw = c"def:Keyword".as_ptr();
    let mut out = vec![
        (std::ptr::null_mut(), 0, 0, 1, kw),
        (ctx, 0, 0, 1, std::ptr::null()),
        (ctx, 99, 0, 1, kw),
        (ctx, 0, 0, 1, BAD_UTF8.as_ptr().cast()),
        (ctx, 0, -1, 2, kw),
        (ctx, 0, 2, 2, kw),
    ];
    if first.is_some_and(|line| !line.to_bytes().is_empty()) {
        out.push((ctx, 0, 0, 2, c"def:Symbol".as_ptr()));
        out.push((ctx, 0, 3, 4, c"def:Unmapped".as_ptr()));
        out.push((ctx, 0, 5, 7, c"def:Number".as_ptr()));
    }
    out
}

struct ApiGuard;

impl Drop for ApiGuard {
    fn drop(&mut self) {
        catalog::TEST_SHARED.with(|cell| cell.set(None));
        TEST_API.with(|cell| cell.set(None));
    }
}

fn with_fake_api<R>(run: impl FnOnce() -> R) -> R {
    TEST_API.with(|cell| cell.set(Some(fake_ffi::API)));
    let _guard = ApiGuard;
    run()
}

fn with_fake_shared<R>(run: impl FnOnce() -> R) -> R {
    with_fake_api(|| {
        let engine = FAKE_SHARED.get_or_init(|| {
            Colorer::open(Path::new("fake-catalog.xml")).expect("fake open succeeds")
        });
        catalog::TEST_SHARED.with(|cell| cell.set(Some(engine)));
        run()
    })
}

#[test]
fn open_and_drop_handle_missing_invalid_and_live_catalogs() {
    assert!(Colorer::open(Path::new("/definitely/missing/catalog.xml")).is_none());
    drop(Colorer::discover());
    with_fake_api(|| {
        assert!(Colorer::open(Path::new("bad\0catalog.xml")).is_none());
        assert!(Colorer::open(Path::new("null.xml")).is_none());
        let before = CLOSES.load(Ordering::Relaxed);
        {
            let engine = Colorer::open(Path::new("valid.xml")).expect("opened");
            assert_eq!(engine.catalog(), Path::new("valid.xml"));
        }
        assert!(CLOSES.load(Ordering::Relaxed) > before);
    });
}

#[test]
fn discover_in_checks_explicit_system_and_home_catalogs() {
    with_fake_api(|| {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let explicit = dir.path().join("explicit.xml");
        std::fs::write(&explicit, "<catalog/>").expect("write");
        let found = Colorer::discover_in(Some(explicit.clone()), &[], None).expect("explicit");
        assert_eq!(found.catalog(), explicit.as_path());

        let candidate = dir.path().join("system.xml");
        std::fs::write(&candidate, "<catalog/>").expect("write");
        let cand_str = candidate.to_str().expect("utf8");
        let sys = Colorer::discover_in(Some(dir.path().join("none.xml")), &[cand_str], None)
            .expect("candidate");
        assert_eq!(sys.catalog(), candidate.as_path());

        let home_cat = dir.path().join(".local/share/ffm/colorer/catalog.xml");
        std::fs::create_dir_all(home_cat.parent().expect("parent")).expect("mkdir");
        std::fs::write(&home_cat, "<catalog/>").expect("write");
        let from_home =
            Colorer::discover_in(None, &[], Some(dir.path().to_path_buf())).expect("home");
        assert_eq!(from_home.catalog(), home_cat.as_path());

        assert!(Colorer::discover_in(None, &[], None).is_none());
        let empty = tempfile::TempDir::new().expect("empty");
        assert!(Colorer::discover_in(None, &[], Some(empty.path().to_path_buf())).is_none());
    });
}

#[test]
fn file_type_and_parse_convert_utf16_spans_with_fake_engine() {
    assert_eq!(fake_ffi::parse_without_cb(), 0);
    with_fake_shared(|| {
        let engine = Colorer::shared().expect("fake shared");
        assert!(engine.file_type(Path::new("bad\0.lua")).is_none());
        assert!(engine.file_type(Path::new("unknown.bin")).is_none());
        assert!(engine.file_type(Path::new("plain.zzzz")).is_none());
        assert_eq!(engine.file_type(Path::new("a.lua")).as_deref(), Some("lua"));

        let lines = vec!["🦀 = 42".to_owned(), "nul\0line".to_owned()];
        assert_eq!(engine.parse(Path::new("bad\0.lua"), &lines).len(), 2);
        assert!(engine
            .parse(Path::new("broken.fail"), &lines)
            .iter()
            .all(Vec::is_empty));
        let parsed = engine.parse(Path::new("script.lua"), &lines);
        let expected = vec![
            span(0, 1, Token::Punctuation),
            span(2, 3, Token::Normal),
            span(4, 6, Token::Number),
        ];
        assert_eq!(parsed[0], expected);
    });
}

#[test]
fn highlighter_and_cache_reparse_colorer_backed_documents() {
    with_fake_shared(|| {
        let lines = vec!["🦀 = 42".to_owned(), "x".to_owned()];
        assert!(!Highlighter::for_file(Path::new("notes.zzzz"), &lines).is_active());
        let mut supported = Highlighter::for_file(Path::new("script.lua"), &lines);
        assert!(supported.is_active());
        assert!(supported.continuation().is_none());
        assert_eq!(supported.line(&lines[0])[0], ("🦀", Token::Punctuation));

        let edited = vec!["🦀 = 99".to_owned(), "x".to_owned()];
        let mut cache = HighlightCache::default();
        cache.prepare(Path::new("script.lua"), &lines, 2);
        cache.prepare(Path::new("script.lua"), &edited, 2);
        cache.prepare_revision(Path::new("script.lua"), &lines, 2, 1);
        cache.prepare_revision(Path::new("script.lua"), &edited, 2, 2);
    });
}
