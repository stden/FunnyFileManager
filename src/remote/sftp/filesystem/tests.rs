use super::{replace_with, RemoteError, RemotePath, RenameFlags};
use ssh2::{Error, ErrorCode};

#[test]
fn replacement_retries_only_an_unsupported_native_rename() {
    let path = RemotePath::parse("/target").expect("path");
    let mut calls = Vec::new();
    assert_eq!(
        replace_with(&path, |flags| {
            calls.push(flags);
            if calls.len() == 1 {
                Err(Error::from_errno(ErrorCode::SFTP(8)))
            } else {
                Ok(())
            }
        }),
        Ok(())
    );
    assert_eq!(
        calls,
        [
            RenameFlags::OVERWRITE | RenameFlags::ATOMIC | RenameFlags::NATIVE,
            RenameFlags::OVERWRITE
        ]
    );
}

#[test]
fn a_failed_fallback_preserves_its_path_and_does_not_retry_again() {
    let path = RemotePath::parse("/target").expect("path");
    let mut calls = 0;
    let result = replace_with(&path, |_| {
        calls += 1;
        Err(Error::from_errno(ErrorCode::SFTP(if calls == 1 {
            8
        } else {
            2
        })))
    });
    assert_eq!(calls, 2);
    assert_eq!(result, Err(RemoteError::NotFound(path)));
}

#[test]
fn successful_and_non_retryable_results_make_one_native_request() {
    let path = RemotePath::parse("/target").expect("path");
    let mut calls = 0;
    assert_eq!(
        replace_with(&path, |_| {
            calls += 1;
            Ok(())
        }),
        Ok(())
    );
    assert_eq!(calls, 1);
    calls = 0;
    assert_eq!(
        replace_with(&path, |_| {
            calls += 1;
            Err(Error::from_errno(ErrorCode::SFTP(3)))
        }),
        Err(RemoteError::Failed("remote operation: permission denied"))
    );
    assert_eq!(calls, 1);
}
