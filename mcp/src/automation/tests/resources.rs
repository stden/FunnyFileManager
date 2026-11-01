//! Purpose: Bound session resources and prove closing releases a reserved slot.

use super::*;

const READY: &str = "{\"ok\":true,\"result\":{\"protocol_version\":1,\"state\":{}}}\n";
const RESULT: &str = "{\"ok\":true,\"result\":{}}\n";

#[tokio::test]
async fn a_missing_ffm_binary_never_consumes_a_session_slot() {
    let root = tempfile::tempdir().unwrap();
    let launcher = launch::BinaryLauncher(root.path().join("absent-ffm").into_os_string());
    let manager = SessionManager::with_launcher(Arc::new(launcher));
    for _ in 0..5 {
        let result = manager
            .start(root.path(), Size::new(80, 24).unwrap(), Profile::Isolated)
            .await;
        assert!(
            matches!(result, Err(Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound)
        );
    }
    assert!(manager.sessions.lock().await.is_empty());
    drop(manager);
}

#[tokio::test]
async fn closing_a_session_releases_its_capacity_and_rejects_its_old_id() {
    let root = tempfile::tempdir().unwrap();
    let manager = manager(READY, RESULT);
    let size = Size::new(80, 24).unwrap();
    let mut sessions = Vec::new();
    for _ in 0..4 {
        let started = manager
            .start(root.path(), size, Profile::Isolated)
            .await
            .unwrap();
        sessions.push(started.session_id);
    }
    assert!(matches!(
        manager.start(root.path(), size, Profile::System).await,
        Err(Error::SessionLimit)
    ));
    let closed = sessions.remove(0);
    manager.close(&closed).await.unwrap();
    assert!(matches!(
        manager.request(&closed, Request::State).await,
        Err(Error::UnknownSession)
    ));
    let replacement = manager
        .start(root.path(), size, Profile::System)
        .await
        .unwrap();
    assert_ne!(replacement.session_id, closed);
    drop(manager);
}

#[test]
fn only_the_isolated_profile_adds_the_fixed_profile_flag() {
    let root = tempfile::tempdir().unwrap();
    let size = Size::new(80, 24).unwrap();
    let binary = std::ffi::OsString::from("ffm");
    let system = launch::command(&binary, root.path(), size, Profile::System);
    let isolated = launch::command(&binary, root.path(), size, Profile::Isolated);
    let args: Vec<_> = isolated.as_std().get_args().collect();
    assert_eq!(
        args.first().copied(),
        Some(std::ffi::OsStr::new("--automation"))
    );
    assert!(args.contains(&std::ffi::OsStr::new("--isolated")));
    drop(args);
    assert!(!system.as_std().get_args().any(|arg| arg == "--isolated"));
    drop(system);
    assert_eq!(isolated.as_std().get_current_dir(), Some(root.path()));
    assert!(isolated
        .as_std()
        .get_envs()
        .any(|(name, value)| name == "FFM_LOG" && value == Some(std::ffi::OsStr::new("0"))));
    drop(isolated);
}
