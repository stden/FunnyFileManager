//! Purpose: What a listing or a process reader asks an [`Owners`] cache — a
//! file's owner and group, a uid's name — and forgetting failed lookups.

use super::{cached, user_name, Lookup, Owners, Uid};
#[cfg(unix)]
use super::{group_name, identifiers, Gid};

impl Owners {
    /// The owner's name for `metadata`, or `None` off Unix and for uids
    /// without a passwd entry.
    #[cfg(unix)]
    pub fn of(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (Uid(uid), _) = identifiers(metadata);
        self.name_of_uid(uid)
    }

    /// The owner's name for `metadata`, or `None` off Unix and for uids
    /// without a passwd entry.
    #[cfg(not(unix))]
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "matches the Unix signature where Owners populates its cache"
    )]
    pub fn of(&mut self, _metadata: &std::fs::Metadata) -> Option<String> {
        let _ = &self.by_uid;
        None
    }

    /// The user name of `uid`, or `None` off Unix and for uids without a
    /// passwd entry. For callers that know a uid but have no file to ask,
    /// such as a process's effective uid.
    #[cfg_attr(
        not(any(unix, test)),
        expect(dead_code, reason = "only Unix callers and unit tests resolve uids")
    )]
    pub fn name_of_uid(&mut self, uid: u32) -> Option<String> {
        cached(&mut self.by_uid, Uid(uid), user_name)
    }

    /// The group's name for `metadata`, or `None` off Unix and for gids
    /// without a group entry.
    #[cfg(unix)]
    pub fn group_of(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (_, gid) = identifiers(metadata);
        cached(&mut self.by_gid, gid, group_name)
    }

    /// The group's name for `metadata`, or `None` off Unix and for gids
    /// without a group entry.
    #[cfg(not(unix))]
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "matches the Unix signature where Owners populates its cache"
    )]
    pub fn group_of(&mut self, _metadata: &std::fs::Metadata) -> Option<String> {
        let _ = &self.by_gid;
        None
    }

    /// `user:group` by name, for the File owners column; an id without a
    /// passwd or group entry shows as its number. `None` off Unix.
    #[cfg(unix)]
    #[expect(
        clippy::unnecessary_wraps,
        reason = "matches the non-Unix signature where Owners returns None"
    )]
    pub fn user_and_group(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (Uid(uid), Gid(gid)) = identifiers(metadata);
        let user = self.of(metadata).unwrap_or_else(|| uid.to_string());
        let group = self.group_of(metadata).unwrap_or_else(|| gid.to_string());
        Some(format!("{user}:{group}"))
    }

    /// `user:group` by name, for the File owners column; an id without a
    /// passwd or group entry shows as its number. `None` off Unix.
    #[cfg(not(unix))]
    #[cfg_attr(
        not(test),
        expect(dead_code, reason = "Windows resolves ACL owners via windows::owners")
    )]
    #[expect(
        clippy::needless_pass_by_ref_mut,
        reason = "matches the Unix signature where Owners populates its cache"
    )]
    pub fn user_and_group(&mut self, _metadata: &std::fs::Metadata) -> Option<String> {
        let _ = &self.by_uid;
        None
    }

    /// Forgets the ids whose lookup failed, so the next ask tries again.
    #[cfg_attr(
        not(any(target_os = "linux", test)),
        expect(
            dead_code,
            reason = "only the Linux process reader calls it; other platforms have no reader yet"
        )
    )]
    pub fn forget_failed_lookups(&mut self) {
        self.by_uid.retain(|_, answer| *answer != Lookup::Failed);
        self.by_gid.retain(|_, answer| *answer != Lookup::Failed);
    }
}

#[cfg(all(test, unix))]
impl Owners {
    /// Owners whose lookup of `uid` has failed, as an NSS or LDAP outage
    /// leaves them: until the failure is forgotten, `uid` has no name.
    pub(crate) fn with_failed_user(uid: u32) -> Self {
        let mut owners = Self::default();
        owners.by_uid.insert(Uid(uid), Lookup::Failed);
        owners
    }
}
