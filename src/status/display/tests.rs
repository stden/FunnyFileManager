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
