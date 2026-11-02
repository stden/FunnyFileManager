//! Purpose: What the `kbdx` tool refuses: paths outside the root, bad inputs,
//! missing credentials and oversized vaults — without leaking secrets.

use super::*;

#[test]
fn database_and_keyfile_must_both_stay_inside_the_server_root() {
    let root_dir = TempDir::new().expect("root temp dir");
    let outside_dir = TempDir::new().expect("outside temp dir");
    let (server, root) = server_at(&root_dir);
    let inside_vault = root.join("inside.kdbx");
    let outside_vault = outside_dir.path().join("outside.kdbx");
    let outside_key = outside_dir.path().join("outside.key");
    save_password_fixture(&inside_vault);
    save_password_fixture(&outside_vault);
    File::create(&outside_key).expect("outside key fixture");

    let vault_error = server
        .kbdx(Parameters(params(&outside_vault, KdbxFormat::Json)))
        .expect_err("outside database must be refused");
    assert!(vault_error.message.contains("outside the allowed root"));

    let key_error = server
        .kbdx(Parameters(KdbxParams {
            path: inside_vault.display().to_string(),
            password: Some(MASTER_PASSWORD.to_owned()),
            keyfile: Some(outside_key.display().to_string()),
            format: KdbxFormat::Json,
        }))
        .expect_err("outside key file must be refused");
    assert!(key_error.message.contains("outside the allowed root"));
}

#[test]
fn invalid_inputs_do_not_leak_credentials() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("vault.kdbx");
    save_password_fixture(&vault);
    let wrong_password = "this-must-not-appear-in-the-error";

    let error = server
        .kbdx(Parameters(KdbxParams {
            path: vault.display().to_string(),
            password: Some(wrong_password.to_owned()),
            keyfile: None,
            format: KdbxFormat::Json,
        }))
        .expect_err("wrong password must fail");

    assert!(error.message.contains("could not decrypt"));
    assert!(!error.message.contains(wrong_password));
}

#[test]
fn a_credential_and_a_vault_file_extension_are_required() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let text = root.join("not-a-vault.txt");
    File::create(&text).expect("text fixture");

    let credential_error = server
        .kbdx(Parameters(KdbxParams {
            path: text.display().to_string(),
            password: None,
            keyfile: None,
            format: KdbxFormat::Json,
        }))
        .expect_err("credential-less request must fail first");
    assert!(credential_error.message.contains("provide a password"));

    let extension_error = server
        .kbdx(Parameters(KdbxParams {
            path: text.display().to_string(),
            password: Some(MASTER_PASSWORD.to_owned()),
            keyfile: None,
            format: KdbxFormat::Json,
        }))
        .expect_err("non-vault extension must fail");
    assert!(extension_error.message.contains(".kdbx or .kbdx"));
}

#[test]
fn oversized_database_is_rejected_before_decryption() {
    let dir = TempDir::new().expect("temp dir");
    let (server, root) = server_at(&dir);
    let vault = root.join("large.kdbx");
    File::create(&vault)
        .expect("create sparse fixture")
        .set_len(kdbx::MAX_DATABASE_BYTES + 1)
        .expect("size sparse fixture");

    let error = server
        .kbdx(Parameters(params(&vault, KdbxFormat::Json)))
        .expect_err("oversized database must be refused");

    assert!(error.message.contains("maximum"));
    assert!(!error.message.contains(MASTER_PASSWORD));
}
