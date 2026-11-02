//! Purpose: What a listing or a process reader asks an [`Owners`] cache — a
//! file's owner and group, a uid's name — and forgetting failed lookups.

use super::{cached, group_name, identifiers, user_name, Gid, Lookup, Owners, Uid};

impl Owners {
    /// The owner's name for `metadata`, or `None` off Unix and for uids
    /// without a passwd entry.
    pub fn of(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (Uid(uid), _) = identifiers(metadata)?;
        self.name_of_uid(uid)
    }

    /// The user name of `uid`, or `None` off Unix and for uids without a
    /// passwd entry. For callers that know a uid but have no file to ask,
    /// such as a process's effective uid.
    pub fn name_of_uid(&mut self, uid: u32) -> Option<String> {
        cached(&mut self.by_uid, Uid(uid), user_name)
    }

    /// The group's name for `metadata`, or `None` off Unix and for gids
    /// without a group entry.
    pub fn group_of(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (_, gid) = identifiers(metadata)?;
        cached(&mut self.by_gid, gid, group_name)
    }

    /// `user:group` by name, for the File owners column; an id without a
    /// passwd or group entry shows as its number. `None` off Unix.
    pub fn user_and_group(&mut self, metadata: &std::fs::Metadata) -> Option<String> {
        let (Uid(uid), Gid(gid)) = identifiers(metadata)?;
        let user = self.of(metadata).unwrap_or_else(|| uid.to_string());
        let group = self.group_of(metadata).unwrap_or_else(|| gid.to_string());
        Some(format!("{user}:{group}"))
    }

    /// Forgets the ids whose lookup failed, so the next ask tries again.
    ///
    /// Until then a failed id is not asked again, like a named one: a listing
    /// never calls this, so an LDAP outage costs one lookup per id, not one per
    /// row. A reader that lives for the session calls it once per sample, so
    /// one transient NSS error does not leave a bare uid on screen until the
    /// session ends.
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
