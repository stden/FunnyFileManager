//! Opt-in smoke test for a user-supplied `KeePass` vault.
//!
//! The repository never names or locates a real vault. To run this test, set
//! `FFM_TEST_KEEPASS_VAULT` and `FFM_TEST_KEEPASS_PASSWORD`, then use:
//!
//! `cargo test --features keepass --lib tests_external_vault -- --ignored`

use std::{env, path::PathBuf};

use crate::kdbx::MasterKey;

#[test]
#[ignore = "requires FFM_TEST_KEEPASS_VAULT and FFM_TEST_KEEPASS_PASSWORD"]
fn configured_external_vault_can_be_decrypted_and_read() {
    let path = required_vault_path();
    let password = required_password();
    super::list_entries(&path, password.expose(), None)
        .unwrap_or_else(|error| panic!("could not read configured KeePass vault: {error}"));
}

fn required_vault_path() -> PathBuf {
    env::var_os("FFM_TEST_KEEPASS_VAULT")
        .map(PathBuf::from)
        .expect("FFM_TEST_KEEPASS_VAULT must name the external test vault")
}

fn required_password() -> MasterKey {
    MasterKey::new(
        env::var("FFM_TEST_KEEPASS_PASSWORD")
            .expect("FFM_TEST_KEEPASS_PASSWORD must unlock the external test vault"),
    )
}
