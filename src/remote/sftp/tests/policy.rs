use super::super::{authentication_result, host_key_decision, ssh_result};
use crate::remote::RemoteError;
use ssh2::{CheckResult, Error, ErrorCode};

#[test]
fn native_authentication_success_still_requires_an_authenticated_session() {
    assert_eq!(authentication_result(Ok(()), true), Ok(()));
    assert_eq!(
        authentication_result(Ok(()), false),
        Err(RemoteError::Failed("SSH authentication failed"))
    );
    let error = Error::new(ErrorCode::Session(-18), "fixture-private-protocol-details");
    assert_eq!(
        authentication_result(Err(error), true),
        Err(RemoteError::Failed("SSH authentication failed"))
    );
}

#[test]
fn confirmation_accepts_only_the_exact_new_key_and_never_overrides_mismatch() {
    let key = "SHA256:fixture";
    assert_eq!(
        host_key_decision(CheckResult::Match, key.to_owned(), None),
        Ok(())
    );
    for trusted in [None, Some("SHA256:different")] {
        assert_eq!(
            host_key_decision(CheckResult::NotFound, key.to_owned(), trusted),
            Err(RemoteError::HostKey(key.to_owned()))
        );
    }
    assert_eq!(
        host_key_decision(CheckResult::NotFound, key.to_owned(), Some(key)),
        Ok(())
    );
    assert_eq!(
        host_key_decision(CheckResult::Mismatch, key.to_owned(), Some(key)),
        Err(RemoteError::Failed(
            "SSH host key changed; connection refused"
        ))
    );
    assert_eq!(
        host_key_decision(CheckResult::Failure, key.to_owned(), Some(key)),
        Err(RemoteError::Failed("cannot verify SSH host key"))
    );
}

fn assert_redacted<T>() {
    let private = "fixture-private-protocol-details";
    let native = Error::new(ErrorCode::Session(-18), private);
    let translated = ssh_result::<T>(Err(native), "fixture SSH stage failed")
        .err()
        .expect("native failure translated");
    assert_eq!(translated, RemoteError::Failed("fixture SSH stage failed"));
    assert!(!translated.to_string().contains(private));
}

#[test]
fn native_error_translation_redacts_details_for_every_transport_result_type() {
    assert_redacted::<ssh2::Session>();
    assert_redacted::<ssh2::Sftp>();
    assert_redacted::<ssh2::KnownHosts>();
    assert_redacted::<u32>();
    assert_redacted::<()>();
    assert_eq!(ssh_result(Ok(7_u32), "unused failure"), Ok(7));
}
