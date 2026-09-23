//! Purpose: uid → user name and gid → group name, looked up once each per listing.
//!
//! `getpwuid_r` and `getgrgid_r` are NSS calls — they can hit LDAP — so a
//! directory of ten thousand files owned by three users asks three times, not
//! ten thousand. The cache dies with the listing: a renamed user is a reload
//! away.

use std::collections::HashMap;

/// Names already resolved while reading one directory or one selection.
#[derive(Default)]
pub struct Owners {
    by_uid: HashMap<u32, Option<String>>,
    by_gid: HashMap<u32, Option<String>>,
}

impl Owners {
    /// The owner's name for `metadata`, or `None` off Unix and for uids
    /// without a passwd entry.
    pub fn of(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (uid, _) = identifiers(metadata)?;
        self.by_uid
            .entry(uid)
            .or_insert_with(|| user_name(uid))
            .clone()
    }

    /// The group's name for `metadata`, or `None` off Unix and for gids
    /// without a group entry.
    pub fn group_of(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (_, gid) = identifiers(metadata)?;
        self.by_gid
            .entry(gid)
            .or_insert_with(|| group_name(gid))
            .clone()
    }

    /// `user:group` by name, for the File owners column; an id without a
    /// passwd or group entry shows as its number. `None` off Unix.
    pub fn user_and_group(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (uid, gid) = identifiers(metadata)?;
        let user = self.of(metadata).unwrap_or_else(|| uid.to_string());
        let group = self.group_of(metadata).unwrap_or_else(|| gid.to_string());
        Some(format!("{user}:{group}"))
    }
}

#[cfg(unix)]
#[expect(
    clippy::unnecessary_wraps,
    reason = "shares the signature of the non-Unix variant, which has no ids to return"
)]
fn identifiers(metadata: &std::fs::Metadata) -> Option<(u32, u32)> {
    use std::os::unix::fs::MetadataExt;
    Some((metadata.uid(), metadata.gid()))
}

#[cfg(not(unix))]
fn identifiers(_metadata: &std::fs::Metadata) -> Option<(u32, u32)> {
    None
}

#[cfg(not(unix))]
fn user_name(_uid: u32) -> Option<String> {
    None
}

#[cfg(not(unix))]
fn group_name(_gid: u32) -> Option<String> {
    None
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn user_name(uid: u32) -> Option<String> {
    lookup(
        // SAFETY: see `lookup`; the id is bound here, the storage there.
        |record, buffer, len, found| unsafe { libc::getpwuid_r(uid, record, buffer, len, found) },
        |record: &libc::passwd| record.pw_name,
    )
}

#[cfg(unix)]
#[allow(unsafe_code)]
fn group_name(gid: u32) -> Option<String> {
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
#[allow(unsafe_code)]
fn lookup<T>(
    call: impl Fn(*mut T, *mut std::ffi::c_char, usize, *mut *mut T) -> i32,
    name_of: impl Fn(&T) -> *const std::ffi::c_char,
) -> Option<String> {
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
            return None;
        }
        // SAFETY: `found` is non-null, so the name points into `buffer`.
        let name = unsafe { std::ffi::CStr::from_ptr(name_of(&record)) };
        return Some(name.to_string_lossy().into_owned());
    }
}

#[cfg(test)]
mod tests;
