//! Purpose: Cover the non-vault fallback arm of `Status::format_vault`.

use std::fmt;

use super::Status;

struct VaultFallbackProbe;

impl fmt::Display for VaultFallbackProbe {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        assert!(Status::Cleared.format_vault(f).is_none());
        Ok(())
    }
}

#[test]
fn format_vault_returns_none_for_non_vault_status() {
    assert_eq!(VaultFallbackProbe.to_string(), "");
}

#[test]
fn vault_statuses_render_their_messages_in_all_feature_sets() {
    assert_eq!(
        Status::VaultRemembered.to_string(),
        "Vault closed; key remembered for this session (5 min idle)"
    );
    assert_eq!(
        Status::VaultEntryDeleted.to_string(),
        "KeePass entry deleted"
    );
    assert_eq!(
        Status::VaultChangeUndone.to_string(),
        "Last KeePass change undone"
    );
}
