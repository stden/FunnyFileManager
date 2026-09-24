//! Purpose: uid → user name and gid → group name, looked up once each per cache.
//!
//! `getpwuid_r` and `getgrgid_r` are NSS calls — they can hit LDAP — so a
//! directory of ten thousand files owned by three users asks three times, not
//! ten thousand. A listing's cache dies with the listing, so a renamed user is
//! a reload away. A process reader keeps its cache for the whole session and
//! calls [`Owners::forget_failed_lookups`] once per sample: a name, or the
//! absence of one, is kept, but a lookup that failed is asked again.

use std::{collections::HashMap, hash::Hash};

mod queries;

/// Names already resolved while reading one directory or one selection.
#[derive(Default)]
pub struct Owners {
    by_uid: HashMap<Uid, Lookup>,
    by_gid: HashMap<Gid, Lookup>,
}

/// A user id. Apart from [`Gid`] so that neither cache, nor either lookup, can
/// be handed the other kind of id: both are a bare `u32` to libc.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Uid(u32);

/// A group id; see [`Uid`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Gid(u32);

/// What one NSS lookup answered for an id.
#[derive(Debug, PartialEq, Eq)]
enum Lookup {
    /// The id's name.
    #[cfg_attr(
        not(unix),
        allow(dead_code, reason = "off Unix there are no ids, so nothing is named")
    )]
    Named(String),
    /// The id has no entry; asking again will not give it one.
    NoEntry,
    /// The lookup itself failed — I/O, file descriptors, memory, an entry
    /// past the buffer cap — so a later ask may succeed.
    Failed,
}

impl Lookup {
    fn name(&self) -> Option<String> {
        match self {
            Self::Named(name) => Some(name.clone()),
            Self::NoEntry | Self::Failed => None,
        }
    }
}

/// The name `cache` holds for `id`, asking `lookup` only the first time. Every
/// answer is kept, a missing entry and a failure included, until
/// [`Owners::forget_failed_lookups`] drops the failures.
fn cached<Id: Copy + Eq + Hash>(
    cache: &mut HashMap<Id, Lookup>,
    id: Id,
    lookup: impl FnOnce(Id) -> Lookup,
) -> Option<String> {
    cache.entry(id).or_insert_with(|| lookup(id)).name()
}

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the signature of the non-Unix variant, which has no ids to return"
)]
fn identifiers(metadata: &std::fs::Metadata) -> Option<(Uid, Gid)> {
    use std::os::unix::fs::MetadataExt;
    Some((Uid(metadata.uid()), Gid(metadata.gid())))
}

#[cfg(not(unix))]
fn identifiers(_metadata: &std::fs::Metadata) -> Option<(Uid, Gid)> {
    None
}

#[cfg(not(unix))]
const fn user_name(_uid: Uid) -> Lookup {
    Lookup::NoEntry
}

#[cfg(not(unix))]
const fn group_name(_gid: Gid) -> Lookup {
    Lookup::NoEntry
}

#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "getpwuid_r has no safe wrapper in std or rustix"
)]
fn user_name(Uid(uid): Uid) -> Lookup {
    lookup(
        // SAFETY: see `lookup`; the id is bound here, the storage there.
        |record, buffer, len, found| unsafe { libc::getpwuid_r(uid, record, buffer, len, found) },
        |record: &libc::passwd| record.pw_name,
    )
}

#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "getgrgid_r has no safe wrapper in std or rustix"
)]
fn group_name(Gid(gid): Gid) -> Lookup {
    lookup(
        // SAFETY: see `lookup`; the id is bound here, the storage there.
        |record, buffer, len, found| unsafe { libc::getgrgid_r(gid, record, buffer, len, found) },
        |record: &libc::group| record.gr_name,
    )
}

/// One reentrant NSS lookup, growing the buffer while glibc reports `ERANGE`.
///
/// `call` is the libc function with its id already bound; `name_of` picks the
/// name field out of the filled record.
#[cfg(unix)]
#[allow(
    unsafe_code,
    reason = "zeroes the C record for the reentrant NSS lookup and reads the name it wrote"
)]
fn lookup<T>(
    call: impl Fn(*mut T, *mut std::ffi::c_char, usize, *mut *mut T) -> i32,
    name_of: impl Fn(&T) -> *const std::ffi::c_char,
) -> Lookup {
    let mut buffer = vec![0_u8; 1024];
    loop {
        // SAFETY: passwd and group are plain C structs for which all-zero is a
        // valid (empty) value; glibc fills them before they are read.
        let mut record: T = unsafe { std::mem::zeroed() };
        let mut found: *mut T = std::ptr::null_mut();
        // Inside `call`: every pointer names live storage sized as passed;
        // glibc writes only within `buffer` and sets `found` to `record` or
        // null.
        let status = call(
            &raw mut record,
            buffer.as_mut_ptr().cast(),
            buffer.len(),
            &raw mut found,
        );
        if status == libc::ERANGE && buffer.len() < 1 << 16 {
            buffer.resize(buffer.len() * 2, 0);
            continue;
        }
        if status != 0 || found.is_null() {
            return unnamed(status);
        }
        // SAFETY: `found` is non-null, so the name points into `buffer`.
        let name = unsafe { std::ffi::CStr::from_ptr(name_of(&record)) };
        return Lookup::Named(name.to_string_lossy().into_owned());
    }
}

/// What a lookup that returned no record means, by the status it returned.
///
/// `getpwuid_r(3)` reports an id with no entry as 0 with a null result, and
/// lists ENOENT, ESRCH, EBADF and EPERM as the same answer on some systems.
/// Anything else is the lookup failing: EIO, EINTR, EMFILE, ENFILE, ENOMEM,
/// or ERANGE once the buffer has reached its cap.
#[cfg(unix)]
const fn unnamed(status: i32) -> Lookup {
    match status {
        0 | libc::ENOENT | libc::ESRCH | libc::EBADF | libc::EPERM => Lookup::NoEntry,
        _ => Lookup::Failed,
    }
}

#[cfg(test)]
mod tests;
